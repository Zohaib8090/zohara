# /root/.bashrc -- Zohara live ISO default for root in bash
#
# Keep it minimal: archiso's `set -e` lives in mkarchiso, not here. We only
# add a coloured prompt and the zohara-* helpers in PATH. Anything fancy
# (aliases, completion, history) is left to the user's dotfiles.

# If not running interactively, don't do anything.
[[ $- != *i* ]] && return

# Coloured prompt: red for root, blue for unprivileged.
PS1='\[\033[01;32m\]\u@\h\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ '

# A few quality-of-life aliases.
alias ll='ls -lh'
alias la='ls -lha'
alias l='ls -CF'
alias grep='grep --color=auto'

# Zohara helpers.
alias install='sudo calamares'
alias welcome='/usr/local/bin/zohara-welcome'
alias settings='zohara-settings'
alias store='zohara-store'
