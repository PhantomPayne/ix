use std::collections::HashMap;

/// Output format for resolved slot values.
#[derive(Debug, Default)]
pub enum OutputFormat {
    /// Space-separated (default). For shell expansion via $().
    #[default]
    Default,
    /// Newline-separated. Good for piping to other tools.
    Lines,
    /// NUL-separated. For xargs -0 and tools that handle filenames with spaces.
    Null,
    /// Quoted with trailing comma, one per line. For pasting into code.
    Paste,
    /// JSON array of full item objects. For scripting and tooling.
    Json,
    /// Custom format string applied to each item, one line per item.
    Template(String),
}

/// Parsed CLI arguments.
#[derive(Debug)]
pub struct Args {
    pub command: Command,
    /// Provider-specific flags passed through context (e.g. "a", "ignored").
    pub flags: HashMap<String, String>,
    /// Output format for resolved values.
    pub output_format: OutputFormat,
    /// Don't write the index to disk.
    pub no_save: bool,
}

#[derive(Debug)]
pub enum Command {
    /// Run a provider and display the index (or auto-detect if None).
    List(Option<&'static str>),
    /// Resolve slots and print raw values to stdout.
    Resolve(Vec<String>),
    /// Interactive TUI picker.
    Pick,
    /// Check staleness, exit 1 if stale.
    Stale,
    /// Print shell integration snippet.
    ShellInit,
    /// Run a passthrough command with @ slot expansion.
    Passthrough { cmd: String, args: Vec<String> },
    /// Run a command once per resolved slot.
    Do { slots_arg: String, cmd: String, args: Vec<String> },
}

const SUBCOMMANDS: &[(&str, &str)] = &[
    ("gs",  "git-status"),
    ("gb",  "git-branches"),
    ("gst", "git-stash"),
    ("ps",  "ps"),
    ("ls",  "ls"),
    ("dk",  "docker"),
];

/// Returns true if s looks like a slot selector (starts with a digit).
fn looks_like_slot_selector(s: &str) -> bool {
    s.chars().next().map_or(false, |c| c.is_ascii_digit())
}

pub fn parse(argv: &[String]) -> Result<Args, String> {
    let mut flags: HashMap<String, String> = HashMap::new();
    let mut output_format = OutputFormat::Default;
    let mut no_save = false;
    let mut i = 0;

    // Phase 1: consume leading ix/output flags (before the first positional arg).
    while i < argv.len() {
        let arg = argv[i].as_str();
        match arg {
            "--shell-init" => {
                return Ok(Args { command: Command::ShellInit, flags, output_format, no_save });
            }
            "--pick" => {
                return Ok(Args { command: Command::Pick, flags, output_format, no_save });
            }
            "--stale" => {
                return Ok(Args { command: Command::Stale, flags, output_format, no_save });
            }
            "--paste" | "-p" => { output_format = OutputFormat::Paste; }
            "--lines" | "-l" => { output_format = OutputFormat::Lines; }
            "--null" | "-n" => { output_format = OutputFormat::Null; }
            "--json" | "-j" => { output_format = OutputFormat::Json; }
            "--template" | "-t" => {
                i += 1;
                let val = argv.get(i)
                    .ok_or_else(|| "--template / -t requires a value".to_string())?
                    .clone();
                output_format = OutputFormat::Template(val);
            }
            // -ns is the two-char shorthand for --no-save; must be checked before
            // the generic short-flag expander that would split it into -n and -s.
            "--no-save" | "-ns" => { no_save = true; }
            s if s.starts_with("--") => {
                let key = s.trim_start_matches('-').to_string();
                flags.insert(key, String::new());
            }
            s if s.starts_with('-') && s.len() > 1 => {
                // Single-char provider flags: -a, -A, etc.
                for c in s[1..].chars() {
                    flags.insert(c.to_string(), String::new());
                }
            }
            _ => break, // first positional found
        }
        i += 1;
    }

    if i >= argv.len() {
        // All flags, no positionals → auto-detect list.
        return Ok(Args { command: Command::List(None), flags, output_format, no_save });
    }

    let first = argv[i].as_str();

    // "do" is a reserved subcommand — never treated as passthrough.
    if first == "do" {
        i += 1;
        let slots_arg = argv.get(i)
            .ok_or_else(|| "ix do: expected slot argument (e.g. @1-3)".to_string())?
            .clone();
        i += 1;
        let cmd = argv.get(i)
            .ok_or_else(|| "ix do: expected command after slot argument".to_string())?
            .clone();
        i += 1;
        let args = argv[i..].to_vec();
        return Ok(Args {
            command: Command::Do { slots_arg, cmd, args },
            flags,
            output_format,
            no_save,
        });
    }

    // Known provider subcommand.
    if let Some(&(_, provider)) = SUBCOMMANDS.iter().find(|(cmd, _)| *cmd == first) {
        i += 1;
        // Parse remaining args for trailing ix flags (e.g. `ix gs --no-save`).
        while i < argv.len() {
            let arg = argv[i].as_str();
            match arg {
                "--paste" | "-p" => { output_format = OutputFormat::Paste; }
                "--lines" | "-l" => { output_format = OutputFormat::Lines; }
                "--null" | "-n" => { output_format = OutputFormat::Null; }
                "--json" | "-j" => { output_format = OutputFormat::Json; }
                "--template" | "-t" => {
                    i += 1;
                    let val = argv.get(i)
                        .ok_or_else(|| "--template / -t requires a value".to_string())?
                        .clone();
                    output_format = OutputFormat::Template(val);
                }
                "--no-save" | "-ns" => { no_save = true; }
                s if s.starts_with("--") => {
                    flags.insert(s.trim_start_matches('-').to_string(), String::new());
                }
                s if s.starts_with('-') && s.len() > 1 => {
                    for c in s[1..].chars() {
                        flags.insert(c.to_string(), String::new());
                    }
                }
                _ => {} // ignore unexpected positionals after subcommand
            }
            i += 1;
        }
        return Ok(Args {
            command: Command::List(Some(provider)),
            flags,
            output_format,
            no_save,
        });
    }

    // Slot selectors start with a digit.
    if looks_like_slot_selector(first) {
        let mut slot_args = vec![first.to_string()];
        i += 1;
        while i < argv.len() {
            let arg = argv[i].as_str();
            match arg {
                "--paste" | "-p" => { output_format = OutputFormat::Paste; }
                "--lines" | "-l" => { output_format = OutputFormat::Lines; }
                "--null" | "-n" => { output_format = OutputFormat::Null; }
                "--json" | "-j" => { output_format = OutputFormat::Json; }
                "--template" | "-t" => {
                    i += 1;
                    let val = argv.get(i)
                        .ok_or_else(|| "--template / -t requires a value".to_string())?
                        .clone();
                    output_format = OutputFormat::Template(val);
                }
                "--no-save" | "-ns" => { no_save = true; }
                s if s.starts_with('-') => {
                    if s.starts_with("--") {
                        flags.insert(s.trim_start_matches('-').to_string(), String::new());
                    } else {
                        for c in s[1..].chars() {
                            flags.insert(c.to_string(), String::new());
                        }
                    }
                }
                other => slot_args.push(other.to_string()),
            }
            i += 1;
        }
        return Ok(Args {
            command: Command::Resolve(slot_args),
            flags,
            output_format,
            no_save,
        });
    }

    // Passthrough mode: first positional is an unknown command name.
    // Everything from the command name onwards is passed through as-is
    // (no further ix flag parsing — dashes belong to the subprocess).
    let cmd = first.to_string();
    i += 1;
    let args = argv[i..].to_vec();
    Ok(Args {
        command: Command::Passthrough { cmd, args },
        flags,
        output_format,
        no_save,
    })
}
