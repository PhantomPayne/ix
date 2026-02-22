/// Shell integration snippet for bash and zsh.
pub const SHELL_INIT: &str = r#"
# ix shell integration
# Add to ~/.bashrc or ~/.zshrc

if [ -n "$ZSH_VERSION" ]; then
    # Zsh integration
    _ix_widget() {
        local selected
        selected=$(ix --pick 2>/dev/null)
        if [ -n "$selected" ]; then
            LBUFFER="${LBUFFER}${selected}"
        fi
        zle reset-prompt
    }
    zle -N _ix_widget
    bindkey '^X' _ix_widget

elif [ -n "$BASH_VERSION" ]; then
    # Bash integration
    _ix_widget() {
        local selected
        selected=$(ix --pick 2>/dev/null)
        if [ -n "$selected" ]; then
            READLINE_LINE="${READLINE_LINE:0:$READLINE_POINT}${selected}${READLINE_LINE:$READLINE_POINT}"
            READLINE_POINT=$(( READLINE_POINT + ${#selected} ))
        fi
    }
    bind -x '"\C-x": _ix_widget'
fi

# Optional: git aliases
# alias gs='ix gs'
# alias gb='ix gb'
"#;
