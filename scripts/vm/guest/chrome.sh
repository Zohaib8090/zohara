#!/bin/bash
# Installs Google Chrome from Flathub the way Zohara Store does, and reports where its launcher is.
sudo flatpak install -y --noninteractive flathub com.google.Chrome > /tmp/chrome-install.log 2>&1
{ tail -5 /tmp/chrome-install.log; ls -l /var/lib/flatpak/exports/bin/ | grep -i chrome; } > /tmp/c.txt 2>&1
curl -s --data-binary @/tmp/c.txt 10.0.2.2:8000/out/c.txt
