#!/bin/bash
{ echo "== pkg"; pacman -Q plasma-keyboard 2>&1; echo "== kwinrc"; kreadconfig6 --file kwinrc --group Wayland --key InputMethod; echo "== introspect"; busctl --user tree org.kde.KWin 2>&1 | grep -i keyboard; busctl --user introspect org.kde.KWin /VirtualKeyboard 2>&1 | head -20; } > /tmp/kb.txt 2>&1
curl -s --data-binary @/tmp/kb.txt 10.0.2.2:8000/out/kb.txt
