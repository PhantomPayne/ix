use std::collections::HashMap;

/// Parsed CLI arguments.
#[derive(Debug)]
pub struct Args {
    pub command: Command,
    pub flags: HashMap<String, String>,
    /// Value from `--pick-input <selector>` (bypasses TUI for testing).
    pub pick_input: Option<String>,
    /// Output format for slot resolution.
    pub output_format: OutputFormat,
}

/// Output format for resolved slot values.
#[derive(Debug, Default, PartialEq)]
pub enum OutputFormat {
    /// Space-separated, shell-quoted (default for use in `$(ix ...)`)
    #[default]
    Default,
    /// Newline-separated, no quoting
    Lines,
    /// NUL-separated, no quoting (for xargs -0)
    Null,
    /// Double-quoted with trailing comma, one per line
    Paste,
    /// JSON array of full item objects
    Json,
    /// Format string per item; variables: {raw} {label} {slot} {status} {group}
    Template(String),
}

#[derive(Debug)]
pub enum Command {
    /// Print shell integration snippet for the given shell (zsh, bash, fish).
    Init(String),
    /// Run a provider and display the index (or auto-detect if None).
    List(Option<&'static str>),
    /// Resolve slots and print raw values to stdout.
    Resolve(Vec<String>),
    /// Interactive TUI picker.
    Pick,
    /// Check staleness, exit 1 if stale.
    Stale,
    /// Print help and exit 0.
    Help,
    /// Print version and exit 0.
    Version,
}

const SUBCOMMANDS: &[(&str, &str)] = &[
    ("gs",  "git-status"),
    ("gb",  "git-branches"),
    ("gst", "git-stash"),
    ("ps",  "ps"),
    ("ls",  "ls"),
    ("dk",  "docker"),
];

pub fn parse(argv: &[String]) -> Result<Args, String> {
    let mut flags: HashMap<String, String> = HashMap::new();
    let mut positional: Vec<String> = Vec::new();
    let mut pick = false;
    let mut stale = false;
    let mut pick_input: Option<String> = None;
    let mut output_format = OutputFormat::Default;

    let mut i = 0;
    while i < argv.len() {
        let arg = &argv[i];
        match arg.as_str() {
            "--pick" => pick = true,
            "--stale" => stale = true,

            "--help" | "-h" => {
                return Ok(Args {
                    command: Command::Help,
                    flags,
                    pick_input: None,
                    output_format: OutputFormat::Default,
                });
            }
            "--version" | "-V" => {
                return Ok(Args {
                    command: Command::Version,
                    flags,
                    pick_input: None,
                    output_format: OutputFormat::Default,
                });
            }

            "--pick-input" => {
                i += 1;
                if i >= argv.len() {
                    return Err("--pick-input requires a value".to_string());
                }
                pick_input = Some(argv[i].clone());
            }

            // Output format flags
            "--lines" | "-l" => output_format = OutputFormat::Lines,
            "--null" | "-n" => output_format = OutputFormat::Null,
            "--paste" | "-p" => output_format = OutputFormat::Paste,
            "--json" | "-j" => output_format = OutputFormat::Json,
            "--template" | "-t" => {
                i += 1;
                if i >= argv.len() {
                    return Err("--template requires a value".to_string());
                }
                output_format = OutputFormat::Template(argv[i].clone());
            }

            "--no-save" => {
                flags.insert("no-save".to_string(), String::new());
            }

            // Legacy flag — kept for backward compat, maps to "init" with no shell
            "--shell-init" => {
                positional.insert(0, "init".to_string());
            }

            s if s.starts_with("--") => {
                let key = s.trim_start_matches('-').to_string();
                flags.insert(key, String::new());
            }
            s if s.starts_with('-') && s.len() > 1 => {
                // Short flags: -a, -A (each char is a separate flag)
                for c in s[1..].chars() {
                    flags.insert(c.to_string(), String::new());
                }
            }

            _ => positional.push(arg.clone()),
        }
        i += 1;
    }

    if stale {
        return Ok(Args { command: Command::Stale, flags, pick_input, output_format });
    }
    if pick {
        return Ok(Args { command: Command::Pick, flags, pick_input, output_format });
    }

    // Handle `ix init <shell>`
    if positional.first().map(|s| s.as_str()) == Some("init") {
        let shell = positional.get(1).cloned().unwrap_or_default();
        return Ok(Args { command: Command::Init(shell), flags, pick_input, output_format });
    }

    if positional.is_empty() {
        // Auto-detect provider
        return Ok(Args { command: Command::List(None), flags, pick_input, output_format });
    }

    // Check if first positional is a known provider subcommand
    if let Some(&(_, provider)) = SUBCOMMANDS.iter().find(|(cmd, _)| *cmd == positional[0].as_str()) {
        return Ok(Args { command: Command::List(Some(provider)), flags, pick_input, output_format });
    }

    // Otherwise treat all positionals as slot selectors
    Ok(Args { command: Command::Resolve(positional), flags, pick_input, output_format })
}
