mod args;
mod display;
mod picker;
mod shell;

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use ix_core::{Index, Provider, Selection};
use ix_core::provider::Context as IxContext;
use ix_docker::DockerProvider;
use ix_fs::FsProvider;
use ix_git::{GitBranchProvider, GitStashProvider, GitStatusProvider};
use ix_sys::ProcessProvider;

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
    let parsed = args::parse(argv)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let ctx = IxContext::new(cwd).with_flags(parsed.flags);

    match parsed.command {
        args::Command::ShellInit => {
            print!("{}", shell::SHELL_INIT);
        }

        args::Command::Stale => {
            let index_path = Index::index_path(&ctx);
            let index = Index::read(&index_path)
                .context("No index found. Run `ix` first.")?;
            if index.is_stale() {
                eprintln!("ix: index is stale ({} seconds old)", index.age_secs());
                std::process::exit(1);
            }
        }

        args::Command::Pick => {
            let index_path = Index::index_path(&ctx);
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
            let index_path = Index::index_path(&ctx);
            let index = Index::read(&index_path)
                .context("No index found. Run `ix` first to build an index.")?;

            if index.is_stale() {
                display::print_stale_warning(index.age_secs());
            }

            let args_str: Vec<&str> = slot_args.iter().map(|s| s.as_str()).collect();
            let sel = Selection::parse(&args_str)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let items = sel.resolve(&index)
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            let raws: Vec<&str> = items.iter().map(|i| i.raw.as_str()).collect();
            print!("{}", raws.join(" "));
        }

        args::Command::List(provider_name) => {
            run_list_command(provider_name, &ctx)?;
        }
    }

    Ok(())
}

fn run_list_command(provider_name: Option<&str>, ctx: &IxContext) -> Result<()> {
    let index_path = Index::index_path(ctx);

    let items = match provider_name {
        Some("git-status") => GitStatusProvider.list(ctx),
        Some("git-branches") => GitBranchProvider.list(ctx),
        Some("git-stash") => GitStashProvider.list(ctx),
        Some("ps") => ProcessProvider.list(ctx),
        Some("ls") => FsProvider.list(ctx),
        Some("docker") => DockerProvider.list(ctx),
        Some(other) => bail!("Unknown provider: {other}"),

        None => {
            // Auto-detect
            if GitStatusProvider::detect(ctx) {
                GitStatusProvider.list(ctx)
            } else {
                FsProvider.list(ctx)
            }
        }
    };

    let items = items.map_err(|e| anyhow::anyhow!("{e}"))?;

    let provider = provider_name.unwrap_or_else(|| {
        if GitStatusProvider::detect(ctx) { "git-status" } else { "ls" }
    });

    let index = Index::new(provider, items);
    index.write(&index_path)?;
    display::print_index(&index);

    Ok(())
}
