#!/bin/bash
# Updates zohara-settings from the channel, then re-runs the start button logo step and reports.
sudo pacman -Sy --noconfirm zohara-settings 2>&1 | tail -3
rm -f ~/.config/zohara/start-icon
zohara-settings --branding; echo "branding exit: $?"
ls -l /usr/share/icons/hicolor/scalable/apps/zohara-start.svg
cat ~/.config/zohara/start-icon
