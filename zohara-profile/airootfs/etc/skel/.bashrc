#
# ~/.bashrc
#

# If not running interactively, don't do anything
[[ $- != *i* ]] && return

alias ls='ls --color=auto'
alias grep='grep --color=auto'
alias ip='ip -color=auto'

# Run fastfetch if available
if command -v fastfetch >/dev/null 2>&1; then
    fastfetch
fi
