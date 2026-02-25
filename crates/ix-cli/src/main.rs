mod args;
mod display;
mod picker;
mod shell;
mod theme;

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use ix_cat::CatProvider;
use ix_core::provider::Context as IxContext;
use ix_core::provider::Provider;
use ix_core::{assign_slots, reorder_by_group, Index, Selection};
use ix_docker::DockerProvider;
use ix_env::EnvProvider;
use ix_fs::{FdProvider, FsProvider};
use ix_git::{GitBranchProvider, GitLogProvider, GitStashProvider, GitStatusProvider, GitWorktreeProvider};
use ix_port::PortProvider;
use ix_ssh::SshProvider;
use ix_stdin::StdinProvider;
use ix_sys::SysProvider;
use args::{Cli, Commands, OutputFormat};
use theme::StatusTheme;

fn main() -> ExitCode {
    // ── Pre-parse: check for `do` verb before clap ─────────────────────
    let raw_args: Vec<String> = std::env::args().collect();
    if let Some(pos) = raw_args[1..].iter().position(|a| a == "do") {
        let pos = pos + 1; // adjust for the skip
        let slots = &raw_args[1..pos];
        let template = &raw_args[pos + 1..];
        return match run_do(slots, template) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("ix: {e}");
                ExitCode::FAILURE
            }
        };
    }

    let cli = Cli::try_parse_from(&raw_args).unwrap_or_else(|e| e.exit());
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ix: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Terminal session identifier used to key the index file.
///
/// Reads `IX_SESSION_ID` (set by `ix shell-init` to `$$`), falling
/// back to the parent process PID so ix works even without shell-init.
fn session_id() -> String {
    std::env::var("IX_SESSION_ID").unwrap_or_else(|_| {
        #[cfg(unix)]
        {
            std::os::unix::process::parent_id().to_string()
        }
        #[cfg(not(unix))]
        {
            "default".to_string()
        }
    })
}

fn run(cli: Cli) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let theme = StatusTheme::from_env();
    let sid = session_id();

    match cli.command {
        Some(Commands::ShellInit) => {
            print!("{}", shell::SHELL_INIT);
        }

        Some(Commands::Pick) => {
            let index_path = Index::index_path(&sid);
            let index = Index::read(&index_path)
                .context("No index found. Run `ix` first to build an index.")?;

            match picker::run_picker(&index, &theme)? {
                Some(selected) if !selected.is_empty() => {
                    print!("{}", selected.join(" "));
                }
                _ => {}
            }
        }

        // ── Providers ──────────────────────────────────────────────────
        Some(Commands::GitStatus { ignored }) => {
            let ctx = IxContext::new(cwd).with_flags(flags_from([("ignored", ignored)]));
            run_provider(&GitStatusProvider::new(), &ctx, &sid, &theme, cli.filter.as_deref())?;
        }
        Some(Commands::GitBranches) => {
            let ctx = IxContext::new(cwd);
            run_provider(&GitBranchProvider::new(), &ctx, &sid, &theme, cli.filter.as_deref())?;
        }
        Some(Commands::GitLog) => {
            let ctx = IxContext::new(cwd);
            run_provider(&GitLogProvider::new(), &ctx, &sid, &theme, cli.filter.as_deref())?;
        }
        Some(Commands::GitStash) => {
            let ctx = IxContext::new(cwd);
            run_provider(&GitStashProvider::new(), &ctx, &sid, &theme, cli.filter.as_deref())?;
        }
        Some(Commands::GitWorktree) => {
            let ctx = IxContext::new(cwd);
            run_provider(&GitWorktreeProvider::new(), &ctx, &sid, &theme, cli.filter.as_deref())?;
        }
        Some(Commands::Ps { all }) => {
            let ctx = IxContext::new(cwd).with_flags(flags_from([("all", all), ("a", all)]));
            run_provider(&SysProvider::new(), &ctx, &sid, &theme, cli.filter.as_deref())?;
        }
        Some(Commands::Ls { hidden, all }) => {
            let ctx = IxContext::new(cwd).with_flags(flags_from([
                ("hidden", hidden),
                ("a", hidden),
                ("all", all),
                ("A", all),
            ]));
            run_provider(&FsProvider::new(), &ctx, &sid, &theme, cli.filter.as_deref())?;
        }
        Some(Commands::Fd { hidden, all }) => {
            let ctx = IxContext::new(cwd).with_flags(flags_from([
                ("hidden", hidden),
                ("a", hidden),
                ("all", all),
                ("A", all),
            ]));
            run_provider(&FdProvider::new(), &ctx, &sid, &theme, cli.filter.as_deref())?;
        }
        Some(Commands::Docker { all }) => {
            let ctx = IxContext::new(cwd).with_flags(flags_from([("all", all), ("a", all)]));
            run_provider(&DockerProvider::new(), &ctx, &sid, &theme, cli.filter.as_deref())?;
        }
        Some(Commands::Env) => {
            let ctx = IxContext::new(cwd);
            run_provider(&EnvProvider::new(), &ctx, &sid, &theme, cli.filter.as_deref())?;
        }
        Some(Commands::Ports { all }) => {
            let ctx = IxContext::new(cwd).with_flags(flags_from([("all", all), ("a", all)]));
            run_provider(&PortProvider::new(), &ctx, &sid, &theme, cli.filter.as_deref())?;
        }
        Some(Commands::Ssh) => {
            let ctx = IxContext::new(cwd);
            run_provider(&SshProvider::new(), &ctx, &sid, &theme, cli.filter.as_deref())?;
        }
        Some(Commands::Stdin) => {
            let ctx = IxContext::new(cwd);
            run_provider(&StdinProvider::new(), &ctx, &sid, &theme, cli.filter.as_deref())?;
        }
        Some(Commands::Cat { file }) => {
            let ctx = IxContext::new(cwd);
            run_provider(&CatProvider::new(file), &ctx, &sid, &theme, cli.filter.as_deref())?;
        }

        // ── Diff ───────────────────────────────────────────────────────
        Some(Commands::Diff) => {
            run_diff(&sid)?;
        }

        // ── External ix-* command ──────────────────────────────────────
        Some(Commands::External(ext_args)) => {
            let cmd_name = ext_args[0].to_string_lossy();
            let bin = format!("ix-{cmd_name}");
            let status = std::process::Command::new(&bin)
                .args(&ext_args[1..])
                .status()
                .with_context(|| format!("{bin} not found"))?;
            if !status.success() {
                anyhow::bail!("{bin} exited with {status}");
            }
        }

        // ── No subcommand: slot resolution or auto-detect ──────────────
        None => {
            let ctx = IxContext::new(cwd);

            if cli.args.is_empty() {
                auto_detect_and_list(&ctx, &sid, &theme, cli.filter.as_deref())?;
            } else {
                let resolved = resolve_slots(&cli.args, &sid)?;
                
                if !cli.run_command.is_empty() {
                    let mut args = cli.run_command.clone();
                    args.extend(resolved.into_iter().map(|item| item.raw.clone()));
                    
                    let status = std::process::Command::new(&args[0])
                        .args(&args[1..])
                        .status()
                        .with_context(|| format!("failed to execute: {}", args[0]))?;
                        
                    std::process::exit(status.code().unwrap_or(1));
                } else {
                    format_output(&resolved, cli.output_format())?;
                }
            }
        }
    }

    Ok(())
}

fn flags_from<const N: usize>(pairs: [(&str, bool); N]) -> HashSet<String> {
    pairs
        .iter()
        .filter(|(_, v)| *v)
        .map(|(k, _)| k.to_string())
        .collect()
}

fn run_provider(
    provider: &dyn Provider,
    ctx: &IxContext,
    session_id: &str,
    theme: &StatusTheme,
    filter: Option<&str>,
) -> Result<()> {
    let index_path = Index::index_path(session_id);
    let items = provider.list(ctx)?;

    // 1. Reorder into visual group order (first seen order)
    let mut items = reorder_by_group(items);

    // 2. Filter by glob pattern if provided
    if let Some(pat) = filter {
        let glob = globset::Glob::new(pat)
            .with_context(|| format!("Invalid glob pattern: {pat}"))?
            .compile_matcher();
        items.retain(|i| glob.is_match(&i.label));
    }

    // 3. Assign sequential slots
    let items = assign_slots(items);

    let index = Index::new(provider.name(), items);
    index.write(&index_path)?;
    display::print_index(&index, theme);
    Ok(())
}

fn auto_detect_and_list(
    ctx: &IxContext,
    session_id: &str,
    theme: &StatusTheme,
    filter: Option<&str>,
) -> Result<()> {
    let providers: Vec<Box<dyn Provider>> = vec![
        Box::new(GitStatusProvider::new()),
        Box::new(FsProvider::new()),
    ];
    let provider = providers
        .iter()
        .find(|p| p.detect(ctx))
        .unwrap_or_else(|| providers.last().unwrap());
    run_provider(provider.as_ref(), ctx, session_id, theme, filter)
}

fn resolve_slots(args: &[String], session_id: &str) -> Result<Vec<ix_core::Item>> {
    let index_path = Index::index_path(session_id);
    let index =
        Index::read(&index_path).context("No index found. Run `ix` first to build an index.")?;

    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let sel = Selection::parse(&args_str)?;
    let items = sel.resolve(&index)?;
    Ok(items.into_iter().cloned().collect())
}

fn format_output(items: &[ix_core::Item], fmt: OutputFormat) -> Result<()> {
    let raws: Vec<&str> = items.iter().map(|i| i.raw.as_str()).collect();
    match fmt {
        OutputFormat::Space => print!("{}", raws.join(" ")),
        OutputFormat::Null => {
            for r in &raws {
                print!("{r}\0");
            }
        }
        OutputFormat::Newline => print!("{}", raws.join("\n")),
        OutputFormat::Paste => {
            let formatted: Vec<String> = raws.iter().map(|r| format!("\"{r}\"")).collect();
            print!("{}", formatted.join(",\n"));
        }
        OutputFormat::Template(tpl) => {
            for r in &raws {
                print!("{}", tpl.replace("{}", r));
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&items)?;
            print!("{json}");
        }
    }
    Ok(())
}

// ── `do` verb ──────────────────────────────────────────────────────────────

fn run_do(slots: &[String], template: &[String]) -> Result<()> {
    if slots.is_empty() {
        anyhow::bail!("usage: ix <slots> do <command...>");
    }
    if template.is_empty() {
        anyhow::bail!("usage: ix <slots> do <command...>");
    }

    let sid = session_id();
    let index_path = Index::index_path(&sid);
    let index =
        Index::read(&index_path).context("No index found. Run `ix` first to build an index.")?;

    let args_str: Vec<&str> = slots.iter().map(|s| s.as_str()).collect();
    let sel = Selection::parse(&args_str)?;
    let items = sel.resolve(&index)?;

    let tpl = template.join(" ");

    for item in &items {
        let expanded = tpl.replace("{}", &item.raw);
        eprintln!("+ {expanded}");
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&expanded)
            .status()
            .with_context(|| format!("failed to execute: {expanded}"))?;
        if !status.success() {
            anyhow::bail!("command exited with {status}: {expanded}");
        }
    }

    Ok(())
}

// ── `diff` ─────────────────────────────────────────────────────────────────

fn run_diff(session_id: &str) -> Result<()> {
    use crossterm::style::{Attribute, Color, Stylize};
    use std::collections::HashSet;

    let index_path = Index::index_path(session_id);
    let prev_path = Index::prev_path(session_id);

    let current = Index::read(&index_path)
        .context("No current index found. Run a provider first (e.g. `ix gs`).")?;
    let previous = Index::read(&prev_path)
        .context("No previous index found. Run a provider at least twice to see a diff.")?;

    let cur_raws: HashSet<&str> = current.items.iter().map(|i| i.raw.as_str()).collect();
    let prev_raws: HashSet<&str> = previous.items.iter().map(|i| i.raw.as_str()).collect();

    let added: Vec<&str> = cur_raws.difference(&prev_raws).copied().collect();
    let removed: Vec<&str> = prev_raws.difference(&cur_raws).copied().collect();

    if added.is_empty() && removed.is_empty() {
        println!(
            "  {}",
            "no changes since last index"
                .with(Color::DarkGrey)
                .attribute(Attribute::Italic)
        );
        return Ok(());
    }

    for r in &removed {
        let prev_item = previous.items.iter().find(|i| i.raw.as_str() == *r);
        let label = prev_item.map(|i| i.label.as_str()).unwrap_or(r);
        println!("  {} {label}", "-".with(Color::Red));
    }
    for a in &added {
        let cur_item = current.items.iter().find(|i| i.raw.as_str() == *a);
        let label = cur_item.map(|i| i.label.as_str()).unwrap_or(a);
        println!("  {} {label}", "+".with(Color::Green));
    }

    Ok(())
}
