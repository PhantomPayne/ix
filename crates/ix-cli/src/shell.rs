/// Return the shell integration snippet for the given shell name.
pub fn snippet_for(shell: &str) -> Result<&'static str, String> {
    match shell {
        "zsh"  => Ok(ZSH_INIT),
        "bash" => Ok(BASH_INIT),
        "fish" => Ok(FISH_INIT),
        "" => Err("Usage: ix init <shell>  (supported: zsh, bash, fish)".to_string()),
        other => Err(format!(
            "Unknown shell: '{other}'. Supported shells: zsh, bash, fish"
        )),
    }
}

/// Zsh integration snippet.
///
/// Usage in ~/.zshrc:
///   eval "$(ix init zsh)"
///
/// Then bind a key:
///   bindkey '^X' _ix_widget   # Ctrl+X
const ZSH_INIT: &str = r#"export IX_SESSION=$$

_ix_widget() {
    local result
    result=$(ix --pick)
    if [[ $? -eq 0 && -n "$result" ]]; then
        LBUFFER="${LBUFFER}${result} "
    fi
    zle reset-prompt
}
zle -N _ix_widget

# Bind a key of your choice, e.g.:
# bindkey '^X' _ix_widget   # Ctrl+X
# bindkey '^[i' _ix_widget  # Alt+I
# bindkey '^[x' _ix_widget  # Alt+X
"#;

/// Bash integration snippet.
///
/// Usage in ~/.bashrc:
///   eval "$(ix init bash)"
///
/// Then bind a key:
///   bind -x '"\C-x": _ix_widget'   # Ctrl+X
const BASH_INIT: &str = r#"export IX_SESSION=$$

_ix_widget() {
    local result
    result=$(ix --pick)
    if [[ $? -eq 0 && -n "$result" ]]; then
        READLINE_LINE="${READLINE_LINE:0:$READLINE_POINT}${result} ${READLINE_LINE:$READLINE_POINT}"
        READLINE_POINT=$(( READLINE_POINT + ${#result} + 1 ))
    fi
}

# Bind a key of your choice, e.g.:
# bind -x '"\C-x": _ix_widget'   # Ctrl+X
# bind -x '"\ei": _ix_widget'    # Alt+I
"#;

/// Fish integration snippet.
///
/// Usage in ~/.config/fish/config.fish:
///   ix init fish | source
///
/// Then bind a key:
///   bind \cx _ix_widget    # Ctrl+X
const FISH_INIT: &str = r#"set -x IX_SESSION %self

function _ix_widget
    set result (ix --pick)
    if test $status -eq 0 -a -n "$result"
        commandline -i "$result "
    end
end

# Bind a key of your choice, e.g.:
# bind \cx _ix_widget    # Ctrl+X
# bind \ei _ix_widget    # Alt+I
"#;
