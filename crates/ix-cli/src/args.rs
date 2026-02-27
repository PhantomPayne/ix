use clap::{ArgGroup, Parser, Subcommand};
use std::ffi::OsString;

/// How resolved slots are printed to stdout.
pub enum OutputFormat {
    /// Space-separated (default): `a b c`
    Space,
    /// Null-terminated (`-0`): `a\0b\0c\0`  — pipe to `xargs -0`
    Null,
    /// Newline-separated (`-n`): `a\nb\nc`
    Newline,
    /// Code-style paste (`-p`): `"a",\n"b",\n"c"`
    Paste,
    /// User template (`-t '{}\n'`): `{}` replaced per item
    Template(String),
    /// Full JSON output (`-j`): serialized Item array
    Json,
}

#[derive(Parser)]
#[command(
    name = "ix",
    version,
    about = "Assign short numeric slots to anything, expand them anywhere.",
    long_about = "Run `ix` to list the current context (git status, processes, etc).\n\
                  Use slot numbers to resolve items: `ix 1 3` prints their raw values."
)]
#[command(group(ArgGroup::new("format").multiple(false)))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    // ── Output format flags (mutually exclusive) ───────────────────────
    /// Null-terminate output (for xargs -0)
    #[arg(short = '0', long = "null", global = true, group = "format")]
    pub null: bool,

    /// Newline-separate output
    #[arg(short = 'n', long = "newline", global = true, group = "format")]
    pub newline: bool,

    /// Code-style paste format ("item",\n)
    #[arg(short = 'p', long = "paste", global = true, group = "format")]
    pub paste: bool,

    /// Custom template ({} is replaced with each item)
    #[arg(short = 't', long = "template", global = true, group = "format")]
    pub template: Option<String>,

    /// Output as JSON array
    #[arg(short = 'j', long = "json", global = true, group = "format")]
    pub json: bool,

    /// Glob pattern to filter items by label (e.g. "*.rs", "src/*")
    #[arg(short = 'f', long = "filter", global = true)]
    pub filter: Option<String>,

    /// Slot selectors (e.g. 1, 1-3, 1,3,5)
    pub args: Vec<String>,

    /// Command to execute with all resolved items appended
    #[arg(
        last = true,
        help = "Command to run with resolved items (e.g. `ix 1-3 -- git add`)"
    )]
    pub run_command: Vec<String>,
}

impl Cli {
    pub fn output_format(&self) -> OutputFormat {
        if self.null {
            OutputFormat::Null
        } else if self.newline {
            OutputFormat::Newline
        } else if self.paste {
            OutputFormat::Paste
        } else if self.json {
            OutputFormat::Json
        } else if let Some(ref tpl) = self.template {
            OutputFormat::Template(tpl.clone())
        } else {
            OutputFormat::Space
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Open the interactive TUI picker
    Pick,
    /// Print the shell integration snippet
    ShellInit,

    // ── Providers ──────────────────────────────────────────────────────
    /// Git status (staged, unstaged, untracked)
    #[command(alias = "gs")]
    GitStatus {
        /// Include gitignored files in the listing
        #[arg(long)]
        ignored: bool,
    },
    /// Git branches (local and remote)
    #[command(alias = "gb")]
    GitBranches,
    /// Git stash entries
    #[command(alias = "gst")]
    GitStash,
    /// Running processes
    #[command(name = "ps")]
    Ps {
        /// Show all users' processes, not just current user
        #[arg(short = 'a', long)]
        all: bool,
    },
    /// File listing
    #[command(name = "ls")]
    Ls {
        /// Include hidden files (dotfiles)
        #[arg(short = 'a', long)]
        hidden: bool,
        /// Include hidden AND gitignored files
        #[arg(short = 'A', long)]
        all: bool,
    },
    /// Docker containers
    #[command(alias = "dk")]
    Docker {
        /// Include stopped containers (default: running only)
        #[arg(short = 'a', long)]
        all: bool,
    },
    /// Environment variables
    #[command(name = "env")]
    Env,
    /// Listening TCP ports
    #[command(name = "port", alias = "ports")]
    Ports {
        /// Include UDP and all users' ports
        #[arg(short = 'a', long)]
        all: bool,
    },
    /// SSH hosts from ~/.ssh/config
    #[command(name = "ssh")]
    Ssh,
    /// Read items from standard input
    #[command(name = "stdin", alias = "-")]
    Stdin,
    /// Show changes since last index
    #[command(name = "diff")]
    Diff,
    /// Read lines from a file
    #[command(name = "cat", alias = "c")]
    Cat {
        /// Target file to read
        file: String,
    },
    /// Git log
    #[command(alias = "gl")]
    GitLog,
    /// Find files recursively
    #[command(name = "fd", alias = "find")]
    Fd {
        /// Include hidden files (dotfiles)
        #[arg(short = 'a', long)]
        hidden: bool,
        /// Include hidden AND gitignored files
        #[arg(short = 'A', long)]
        all: bool,
    },
    /// Git worktrees
    #[command(alias = "wt")]
    GitWorktree,

    /// Run an external ix-* command from PATH
    #[command(external_subcommand)]
    External(Vec<OsString>),
}
