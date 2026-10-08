#!/bin/bash
{ echo "== themes with start-here-kde*"; find /usr/share/icons ~/.local/share/icons -iname "start-here-kde*" 2>/dev/null | head -20; echo "== Fluent-dark index"; grep -n "Inherits" /usr/share/icons/Fluent-dark/index.theme; ls /usr/share/icons | tr '\n' ' '; echo; echo "== places start icons in fluent"; find /usr/share/icons/Fluent* -iname "*start-here*" -o -iname "*windows*" 2>/dev/null | head; } > /tmp/st.txt 2>&1
curl -s --data-binary @/tmp/st.txt 10.0.2.2:8000/out/st.txt
