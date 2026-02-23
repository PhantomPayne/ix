mod args;
mod display;
mod picker;
mod shell;

use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use anyhow::{bail, Result};
use ix_core::provider::Context as IxContext;
use ix_core::{Index, Item, Provider, Selection};
use ix_docker::DockerProvider;
use ix_fs::FsProvider;
use ix_git::{GitBranchProvider, GitStashProvider, GitStatusProvider};
use ix_sys::SysProvider;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match run(&argv) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("ix: {e}");
            std::process::exit(1);
        }
    }
}

// ── Shell quoting ─────────────────────────────────────────────────────────────

/// Shell-quote a string using single quotes.
///
/// Single quotes that appear within the value are escaped as `'\''`.
/// Example: `it's fine` → `'it'\''s fine'`
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

// ── Index path (session-keyed) ────────────────────────────────────────────────

/// Returns the path to the index for the current terminal session.
///
/// Uses `$IX_SESSION` (exported as `$$` by `ix init <shell>`) so that each
/// terminal gets its own isolated index at `/tmp/ix-<session>`.
/// Falls back to the current process PID when `IX_SESSION` is not set.
fn index_path() -> PathBuf {
    Index::session_index_path()
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn run(argv: &[String]) -> Result<()> {
    let parsed = args::parse(argv).map_err(|e| anyhow::anyhow!("{e}"))?;

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let ctx = IxContext::new(cwd).with_flags(parsed.flags);

    // --pick-input bypasses the TUI for non-interactive testing
    if let Some(ref input) = parsed.pick_input {
        return handle_pick_input(input);
    }

    match parsed.command {
        // ── Help / Version ────────────────────────────────────────────────
        args::Command::Help => {
            print!("{}", help_text(&ctx));
        }

        args::Command::Version => {
            println!("ix {}", env!("CARGO_PKG_VERSION"));
        }

        // ── Shell integration snippet ─────────────────────────────────────
        args::Command::Init(shell) => match shell::snippet_for(&shell) {
            Ok(snippet) => print!("{}", snippet),
            Err(e) => {
                eprintln!("ix: {e}");
                std::process::exit(1);
            }
        },

        // ── Staleness check ───────────────────────────────────────────────
        args::Command::Stale => {
            let path = index_path();
            let index = Index::read(&path)
                .map_err(|_| anyhow::anyhow!("No index found. Run 'ix' first."))?;
            if index.is_stale() {
                eprintln!("ix: index is stale ({} seconds old)", index.age_secs());
                std::process::exit(1);
            }
        }

        // ── Interactive TUI picker ────────────────────────────────────────
        args::Command::Pick => {
            let path = index_path();
            let index = match Index::read(&path) {
                Ok(i) => i,
                Err(_) => {
                    eprintln!("  no index found — run 'ix' or 'ix gs' first");
                    std::process::exit(1);
                }
            };

            if index.is_stale() {
                display::print_stale_warning(index.age_secs());
            }

            match picker::run_picker(&index)? {
                Some(selected) if !selected.is_empty() => {
                    let quoted: Vec<String> =
                        selected.iter().map(|s| shell_quote(s)).collect();
                    print!("{}", quoted.join(" "));
                }
                // Empty selection or Esc/Ctrl+C → abort
                _ => {
                    std::process::exit(1);
                }
            }
        }

        // ── Slot resolution ───────────────────────────────────────────────
        args::Command::Resolve(slot_args) => {
            let path = index_path();
            let index = Index::read(&path)
                .map_err(|_| anyhow::anyhow!("No index found. Run 'ix' first to build an index."))?;

            if index.is_stale() {
                display::print_stale_warning(index.age_secs());
            }

            let args_str: Vec<&str> = slot_args.iter().map(|s| s.as_str()).collect();
            let sel = Selection::parse(&args_str)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let items = sel.resolve(&index)
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            let output = format_output(&items, &parsed.output_format);
            print!("{}", output);
        }

        // ── Provider list ─────────────────────────────────────────────────
        args::Command::List(provider_name) => {
            run_list_command(provider_name, &ctx)?;
        }
    }

    Ok(())
}

// ── --pick-input (non-interactive testing helper) ─────────────────────────────

/// Resolve `input` as a slot selector against the current index and print
/// the shell-quoted results, then exit 0.  An empty `input` or a missing
/// index both exit 1 with no output, matching the abort contract.
fn handle_pick_input(input: &str) -> Result<()> {
    if input.is_empty() {
        std::process::exit(1);
    }

    let path = index_path();
    let index = match Index::read(&path) {
        Ok(i) => i,
        Err(_) => std::process::exit(1),
    };

    let sel = match Selection::parse(&[input]) {
        Ok(s) => s,
        Err(e) => bail!("{e}"),
    };
    let items = match sel.resolve(&index) {
        Ok(i) => i,
        Err(e) => bail!("{e}"),
    };

    if items.is_empty() {
        std::process::exit(1);
    }

    let quoted: Vec<String> = items.iter().map(|i| shell_quote(&i.raw)).collect();
    print!("{}", quoted.join(" "));
    Ok(())
}

// ── Output formatting ─────────────────────────────────────────────────────────

fn format_output(items: &[&Item], format: &args::OutputFormat) -> String {
    match format {
        args::OutputFormat::Default => items
            .iter()
            .map(|i| shell_quote(&i.raw))
            .collect::<Vec<_>>()
            .join(" "),

        args::OutputFormat::Lines => items
            .iter()
            .map(|i| i.raw.clone())
            .collect::<Vec<_>>()
            .join("\n"),

        args::OutputFormat::Null => items
            .iter()
            .map(|i| i.raw.clone())
            .collect::<Vec<_>>()
            .join("\0"),

        args::OutputFormat::Paste => items
            .iter()
            .map(|i| format!("\"{}\",", i.raw))
            .collect::<Vec<_>>()
            .join("\n"),

        args::OutputFormat::Json => {
            serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string())
        }

        args::OutputFormat::Template(tmpl) => items
            .iter()
            .map(|i| {
                tmpl.replace("{raw}", &i.raw)
                    .replace("{label}", &i.label)
                    .replace("{slot}", &i.slot.to_string())
                    .replace("{status}", i.status.as_deref().unwrap_or(""))
                    .replace("{group}", i.group.as_deref().unwrap_or(""))
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

// ── Provider execution ────────────────────────────────────────────────────────

fn run_list_command(provider_name: Option<&str>, ctx: &IxContext) -> Result<()> {
    let path = index_path();

    let (items, resolved_name) = match provider_name {
        Some("git-status") => (GitStatusProvider::new().list(ctx), "git-status"),
        Some("git-branches") => (GitBranchProvider::new().list(ctx), "git-branches"),
        Some("git-stash") => (GitStashProvider::new().list(ctx), "git-stash"),
        Some("ps") => (SysProvider::new().list(ctx), "ps"),
        Some("ls") => (FsProvider::new().list(ctx), "ls"),
        Some("docker") => (DockerProvider::new().list(ctx), "docker"),

        Some(other) => {
            // Look for an ix-<name> executable on PATH
            if let Some(provider_path) = find_custom_provider(other) {
                return run_custom_provider(&provider_path, other, ctx);
            }
            bail!(
                "Unknown provider: '{other}'. Run 'ix --help' for available providers."
            );
        }

        None => {
            if GitStatusProvider::detect(ctx) {
                (GitStatusProvider::new().list(ctx), "git-status")
            } else {
                (FsProvider::new().list(ctx), "ls")
            }
        }
    };

    let items = items.map_err(|e| anyhow::anyhow!("{e}"))?;
    let index = Index::new(resolved_name, items);
    index.write(&path)?;
    display::print_index(&index);
    Ok(())
}

/// Find an `ix-<name>` executable on `$PATH`.
fn find_custom_provider(name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let target = format!("ix-{}", name);
    std::env::var_os("PATH").and_then(|path_var| {
        std::env::split_paths(&path_var)
            .map(|dir| dir.join(&target))
            .find(|p| {
                p.is_file()
                    && p.metadata()
                        .map(|m| m.permissions().mode() & 0o111 != 0)
                        .unwrap_or(false)
            })
    })
}

/// Run a custom `ix-<name>` provider, collect its newline-delimited stdout
/// as items, and display the index.
fn run_custom_provider(provider_path: &PathBuf, name: &str, ctx: &IxContext) -> Result<()> {
    let path = index_path();

    let output = ProcessCommand::new(provider_path)
        .current_dir(&ctx.cwd)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run custom provider: {e}"))?;

    if !output.status.success() {
        bail!("Custom provider '{}' exited with non-zero status", name);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let items: Vec<Item> = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, line)| {
            let trimmed = line.trim();
            Item::new(i + 1, trimmed, trimmed)
        })
        .collect();

    let provider_label = format!("ix-{}", name);
    let index = Index::new(&provider_label, items);
    index.write(&path)?;
    display::print_index(&index);
    Ok(())
}

// ── Help text ─────────────────────────────────────────────────────────────────

fn help_text(ctx: &IxContext) -> String {
    let custom_section = build_custom_provider_section(ctx);

    format!(
        r#"ix — Ix eXpander

USAGE:
  ix [PROVIDER] [SLOTS] [OPTIONS]     show/resolve from index
  <cmd> | ix [SLOTS] [OPTIONS]        number any line-oriented input
  ix init <SHELL>                     print shell integration snippet

PROVIDERS:
  (none)    auto-detect (git status if in a git repo)
  gs        git status
  gb        git branches
  gst       git stash
  ps        processes (current user)
  ls        filesystem (current directory)
  dk        docker containers

{custom_section}
PROVIDER OPTIONS:
  ix ls -a / --hidden     include hidden files
  ix ls -A / --all        include hidden + gitignored
  ix gs --ignored         include gitignored files
  ix ps -a / --all        all users' processes
  ix dk -a / --all        include stopped containers

SLOT SYNTAX:
  1          single slot
  1-3        range (inclusive)
  1,3        comma separated
  1,3-5      mixed

OUTPUT OPTIONS:
  -l, --lines       newline separated
  -n, --null        NUL separated (for xargs -0)
  -p, --paste       quoted with trailing comma, one per line
  -j, --json        JSON array of full item objects
  -t, --template X  format string per item
                    variables: {{raw}} {{label}} {{slot}} {{status}} {{group}}

SHELL INTEGRATION:
  eval "$(ix init zsh)"      add to ~/.zshrc
  eval "$(ix init bash)"     add to ~/.bashrc
  ix init fish | source      add to ~/.config/fish/config.fish

  Then bind a key in your rc file:
  bindkey '^X' _ix_widget    zsh example

EXAMPLES:
  ix gs                       show git status with slot numbers
  ix 1 3                      expand slots 1 and 3 (space separated)
  git diff $(ix 1)            diff slot 1
  ix 1-3 --null | xargs -0 grep TODO
  ix 1-3 -t 'import "{{raw}}"'  custom format
"#,
        custom_section = custom_section
    )
}

fn build_custom_provider_section(ctx: &IxContext) -> String {
    use std::os::unix::fs::PermissionsExt;

    let mut providers: Vec<(String, PathBuf)> = Vec::new();

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with("ix-") {
                        let p = entry.path();
                        if p.is_file()
                            && p.metadata()
                                .map(|m| m.permissions().mode() & 0o111 != 0)
                                .unwrap_or(false)
                        {
                            providers.push((name_str.into_owned(), p));
                        }
                    }
                }
            }
        }
    }

    let _ = ctx; // ctx available for future per-project detection

    if providers.is_empty() {
        "  Custom providers: any 'ix-<name>' executable on PATH\n".to_string()
    } else {
        providers.sort_by(|a, b| a.0.cmp(&b.0));
        providers.dedup_by(|a, b| a.0 == b.0);
        let mut s = "  Custom providers found:\n".to_string();
        for (name, path) in &providers {
            s.push_str(&format!("    {}      {}\n", name, path.display()));
        }
        s
    }
}
