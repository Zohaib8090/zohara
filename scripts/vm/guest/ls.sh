#!/bin/bash
{ echo "== color-schemes"; ls ~/.local/share/color-schemes; echo "== icons"; ls ~/.local/share/icons; echo "== kdeglobals"; kreadconfig6 --file kdeglobals --group Icons --key Theme; kreadconfig6 --file kdeglobals --group KDE --key widgetStyle; kreadconfig6 --file kdeglobals --group General --key ColorScheme; echo "== kvantum"; cat ~/.config/Kvantum/kvantum.kvconfig; } > /tmp/ls.txt 2>&1
curl -s --data-binary @/tmp/ls.txt 10.0.2.2:8000/out/ls.txt
