# /root/.zshrc -- Zohara live ISO default for root in zsh
#
# Same as .bashrc but for zsh; the live ISO's root shell is zsh by default.

# If not running interactively, don't do anything.
[[ -o interactive ]] || return

# Coloured prompt.
autoload -U colors && colors
setopt PROMPT_SUBST
PROMPT='%F{green}%n@%m%f:%F{blue}%~%f$ '

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

# History.
HISTSIZE=10000
SAVEHIST=10000
HISTFILE=~/.zsh_history
setopt SHARE_HISTORY
