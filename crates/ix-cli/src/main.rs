mod args;
mod display;
mod format;
mod picker;
mod shell;

use std::io::{BufRead, IsTerminal};
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
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

fn run(argv: &[String]) -> Result<()> {
    let reading_stdin = !std::io::stdin().is_terminal();

    let parsed = args::parse(argv).map_err(|e| anyhow::anyhow!("{e}"))?;

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Destructure to avoid partial-move issues when flags are consumed into ctx.
    let args::Args { command, flags, output_format, no_save } = parsed;
    let ctx = IxContext::new(cwd).with_flags(flags);

    // If stdin is piped, attempt to read items from it.
    // If stdin produces zero items (e.g. /dev/null in CI), fall through to
    // normal provider/resolve mode so existing-index tests still work.
    if reading_stdin {
        let stdin_items = read_stdin_items();
        if !stdin_items.is_empty() {
            return handle_stdin(command, output_format, no_save, &ctx, stdin_items);
        }
    }

    handle_normal(command, output_format, no_save, &ctx)
}

// ---------------------------------------------------------------------------
// Stdin mode
// ---------------------------------------------------------------------------

/// Read all non-empty lines from stdin, stripping trailing whitespace.
fn read_stdin_items() -> Vec<Item> {
    let stdin = std::io::stdin();
    stdin
        .lock()
        .lines()
        .filter_map(|l| l.ok())
        .map(|l| l.trim_end().to_string())
        .filter(|l| !l.is_empty())
        .enumerate()
        .map(|(i, line)| Item::new(i + 1, line.clone(), line))
        .collect()
}

/// Handle the case where stdin has piped data.
///
/// - Display mode (`ix`): show numbered list, save index with provider="stdin".
/// - Inline select (`ix 2`): resolve slots from the piped items, no save.
fn handle_stdin(
    command: args::Command,
    output_format: args::OutputFormat,
    no_save: bool,
    ctx: &IxContext,
    items: Vec<Item>,
) -> Result<()> {
    let is_inline_select = matches!(command, args::Command::Resolve(_));
    let should_save = !is_inline_select && !no_save;

    let index = Index::new("stdin", items);

    if should_save {
        let index_path = Index::index_path(ctx);
        index.write(&index_path)?;
    }

    match command {
        args::Command::Resolve(slot_args) => {
            let args_str: Vec<&str> = slot_args.iter().map(|s| s.as_str()).collect();
            let sel = Selection::parse(&args_str).map_err(|e| anyhow::anyhow!("{e}"))?;
            let resolved = sel.resolve(&index).map_err(|e| anyhow::anyhow!("{e}"))?;
            format::output(&resolved, &output_format);
        }
        _ => {
            // Display mode for List (and any other command piped through stdin).
            display::print_index(&index);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Normal (non-stdin) mode
// ---------------------------------------------------------------------------

fn handle_normal(
    command: args::Command,
    output_format: args::OutputFormat,
    no_save: bool,
    ctx: &IxContext,
) -> Result<()> {
    match command {
        args::Command::ShellInit => {
            print!("{}", shell::SHELL_INIT);
        }

        args::Command::Stale => {
            let index_path = Index::index_path(ctx);
            let index = Index::read(&index_path).context("No index found. Run `ix` first.")?;
            if index.is_stale() {
                eprintln!("ix: index is stale ({} seconds old)", index.age_secs());
                std::process::exit(1);
            }
        }

        args::Command::Pick => {
            let index_path = Index::index_path(ctx);
            let index = Index::read(&index_path)
                .context("No index found. Run `ix` first to build an index.")?;

            if index.is_stale() {
                display::print_stale_warning(index.age_secs());
            }

            match picker::run_picker(&index)? {
                Some(selected) if !selected.is_empty() => {
                    print!("{}", selected.join(" "));
                }
                _ => {}
            }
        }

        args::Command::Resolve(slot_args) => {
            let index_path = Index::index_path(ctx);
            let index = Index::read(&index_path)
                .context("No index found. Run `ix` first to build an index.")?;

            if index.is_stale() {
                display::print_stale_warning(index.age_secs());
            }

            let args_str: Vec<&str> = slot_args.iter().map(|s| s.as_str()).collect();
            let sel = Selection::parse(&args_str).map_err(|e| anyhow::anyhow!("{e}"))?;
            let items = sel.resolve(&index).map_err(|e| anyhow::anyhow!("{e}"))?;
            format::output(&items, &output_format);
        }

        args::Command::List(provider_name) => {
            run_list_command(provider_name, ctx, no_save)?;
        }

        args::Command::Do { slots_arg, cmd, args } => {
            let index_path = Index::index_path(ctx);
            let index = Index::read(&index_path)
                .context("No index found. Run `ix` first to build an index.")?;
            run_do(&slots_arg, &cmd, &args, &index)?;
        }

        args::Command::Passthrough { cmd, args } => {
            let index_path = Index::index_path(ctx);
            let index = Index::read(&index_path)
                .context("No index found. Run `ix` first to build an index.")?;
            run_passthrough(&cmd, &args, &index)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Provider list command
// ---------------------------------------------------------------------------

fn run_list_command(
    provider_name: Option<&str>,
    ctx: &IxContext,
    no_save: bool,
) -> Result<()> {
    let index_path = Index::index_path(ctx);

    let items = match provider_name {
        Some("git-status")  => GitStatusProvider::new().list(ctx),
        Some("git-branches") => GitBranchProvider::new().list(ctx),
        Some("git-stash")   => GitStashProvider::new().list(ctx),
        Some("ps")          => SysProvider::new().list(ctx),
        Some("ls")          => FsProvider::new().list(ctx),
        Some("docker")      => DockerProvider::new().list(ctx),
        Some(other)         => bail!("Unknown provider: {other}"),
        None => {
            if GitStatusProvider::detect(ctx) {
                GitStatusProvider::new().list(ctx)
            } else {
                FsProvider::new().list(ctx)
            }
        }
    };

    let items = items.map_err(|e| anyhow::anyhow!("{e}"))?;

    let provider = provider_name.unwrap_or_else(|| {
        if GitStatusProvider::detect(ctx) { "git-status" } else { "ls" }
    });

    let index = Index::new(provider, items);

    if !no_save {
        index.write(&index_path)?;
    }

    display::print_index(&index);
    Ok(())
}

// ---------------------------------------------------------------------------
// ix do — run a command once per slot
// ---------------------------------------------------------------------------

/// Run `cmd` once for each resolved slot.
///
/// If any arg contains `{}`, the slot value is substituted in-place (string
/// replacement, not just standalone `{}`).  If no arg contains `{}`, the
/// value is appended as the final argument.
///
/// Stops and returns a non-zero exit code on the first failing invocation.
fn run_do(slots_arg: &str, cmd: &str, args: &[String], index: &Index) -> Result<()> {
    let sel_str = slots_arg.trim_start_matches('@');
    let sel = Selection::parse(&[sel_str]).map_err(|e| anyhow::anyhow!("{e}"))?;
    let items = sel.resolve(index).map_err(|e| anyhow::anyhow!("{e}"))?;

    let has_placeholder = args.iter().any(|a| a.contains("{}"));

    for item in items {
        let expanded: Vec<String> = if has_placeholder {
            args.iter().map(|a| a.replace("{}", &item.raw)).collect()
        } else {
            let mut v = args.to_vec();
            v.push(item.raw.clone());
            v
        };

        let status = std::process::Command::new(cmd)
            .args(&expanded)
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("{} failed on {}", cmd, item.raw));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Passthrough mode — expand @ sigil args and run the command
// ---------------------------------------------------------------------------

/// Run `cmd` with its args, expanding any `@<selector>` args to slot values.
///
/// Rules:
/// - `@1`, `@1-3`, `@1,3` — expanded to raw slot values (one arg per value).
/// - `--` — consumed by ix (not forwarded); disables @ expansion for all
///   subsequent args.
/// - Everything else — forwarded literally.
fn run_passthrough(cmd: &str, passthrough_args: &[String], index: &Index) -> Result<()> {
    let mut expanded: Vec<String> = Vec::new();
    let mut no_expand = false;

    for arg in passthrough_args {
        if arg == "--" {
            no_expand = true;
            // Consume -- ; do not pass it to the subprocess.
            continue;
        }

        if !no_expand && arg.starts_with('@') {
            let sel_str = &arg[1..]; // strip the '@'
            let sel = Selection::parse(&[sel_str]).map_err(|e| anyhow::anyhow!("{e}"))?;
            let items = sel.resolve(index).map_err(|e| anyhow::anyhow!("{e}"))?;
            for item in items {
                expanded.push(item.raw.clone());
            }
        } else {
            expanded.push(arg.clone());
        }
    }

    let status = std::process::Command::new(cmd)
        .args(&expanded)
        .status()?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}
