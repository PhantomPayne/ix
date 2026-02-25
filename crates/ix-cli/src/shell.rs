/// Shell integration snippet for bash, zsh, and fish.
pub const SHELL_INIT: &str = r#"
# ix shell integration
# Bash / Zsh: add to ~/.bashrc or ~/.zshrc
# Fish:       ix shell-init | source   (add to ~/.config/fish/config.fish)

if [ -n "$ZSH_VERSION" ]; then
    export IX_SESSION_ID=$$

    # Zsh integration
    _ix_widget() {
        local selected
        selected=$(ix pick 2>/dev/null)
        if [ -n "$selected" ]; then
            LBUFFER="${LBUFFER}${selected}"
        fi
        zle reset-prompt
    }
    zle -N _ix_widget
    bindkey '^X' _ix_widget

    # Optional: Ix numeric expansion (e.g. '@1-3' -> 'file1 file2 file3')
    _ix_expand_widget() {
        local words
        words=(${(z)LBUFFER})
        local word="${words[-1]}"
        if [[ $word =~ ^@([0-9,\-]+)$ ]]; then
            local match="${match[1]}"
            local expanded=$(ix "$match" 2>/dev/null)
            if [ -n "$expanded" ]; then
                LBUFFER="${LBUFFER%$word}${expanded} "
            fi
        fi
        zle reset-prompt
    }
    zle -N _ix_expand_widget
    # bindkey '\ee' _ix_expand_widget  # bind to Alt-E

elif [ -n "$BASH_VERSION" ]; then
    export IX_SESSION_ID=$$

    # Bash integration
    _ix_widget() {
        local selected
        selected=$(ix pick 2>/dev/null)
        if [ -n "$selected" ]; then
            READLINE_LINE="${READLINE_LINE:0:$READLINE_POINT}${selected}${READLINE_LINE:$READLINE_POINT}"
            READLINE_POINT=$(( READLINE_POINT + ${#selected} ))
        fi
    }
    bind -x '"\C-x": _ix_widget'

    # Optional: Ix numeric expansion
    _ix_expand_widget() {
        local left="${READLINE_LINE:0:$READLINE_POINT}"
        if [[ "$left" =~ (@[0-9,\-]+)$ ]]; then
            local match="${BASH_REMATCH[1]}"
            local query="${match:1}"
            local expanded=$(ix "$query" 2>/dev/null)
            if [ -n "$expanded" ]; then
                local prefix="${left%$match}"
                local right="${READLINE_LINE:$READLINE_POINT}"
                READLINE_LINE="${prefix}${expanded} ${right}"
                READLINE_POINT=$(( ${#prefix} + ${#expanded} + 1 ))
            fi
        fi
    }
    # bind -x '"\ee": _ix_expand_widget'  # bind to Alt-E

elif status is-interactive 2>/dev/null; then
    # Fish integration
    eval '
    set -gx IX_SESSION_ID %self
    function _ix_widget
        set -l selected (ix pick 2>/dev/null)
        if test -n "$selected"
            commandline -i -- $selected
        end
        commandline -f repaint
    end
    bind \cx _ix_widget

    # Optional: Ix numeric expansion
    function _ix_expand_widget
        set -l token (commandline -t)
        if string match -q -r "^@[0-9,\-]+$" -- "$token"
            set -l match (string replace -r "^@" "" -- "$token")
            set -l expanded (ix "$match" 2>/dev/null)
            if test -n "$expanded"
                commandline -t -- "$expanded "
            end
        end
        commandline -f repaint
    end
    # bind \ee _ix_expand_widget  # bind to Alt-E
    '
fi

# Optional: git aliases
# alias gs='ix gs'
# alias gb='ix gb'
"#;
