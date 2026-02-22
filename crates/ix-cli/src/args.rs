use std::collections::HashMap;

/// Parsed CLI arguments.
#[derive(Debug)]
pub struct Args {
    pub command: Command,
    pub flags: HashMap<String, String>,
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
    let mut shell_init = false;

    let mut i = 0;
    while i < argv.len() {
        let arg = &argv[i];
        match arg.as_str() {
            "--pick" => pick = true,
            "--stale" => stale = true,
            "--shell-init" => shell_init = true,
            s if s.starts_with("--") => {
                let key = s.trim_start_matches('-').to_string();
                flags.insert(key, String::new());
            }
            s if s.starts_with('-') && s.len() > 1 => {
                // Short flags: -a, -A (each char is a flag)
                for c in s[1..].chars() {
                    flags.insert(c.to_string(), String::new());
                }
            }
            _ => positional.push(arg.clone()),
        }
        i += 1;
    }

    if shell_init {
        return Ok(Args { command: Command::ShellInit, flags });
    }
    if pick {
        return Ok(Args { command: Command::Pick, flags });
    }
    if stale {
        return Ok(Args { command: Command::Stale, flags });
    }

    if positional.is_empty() {
        // Auto-detect
        return Ok(Args { command: Command::List(None), flags });
    }

    // Check if first positional is a subcommand
    if let Some(&(_, provider)) = SUBCOMMANDS.iter().find(|(cmd, _)| *cmd == positional[0].as_str()) {
        return Ok(Args { command: Command::List(Some(provider)), flags });
    }

    // Otherwise treat all positionals as slot selectors
    Ok(Args { command: Command::Resolve(positional), flags })
}
