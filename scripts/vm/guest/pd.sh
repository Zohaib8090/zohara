#!/bin/bash
{ echo "== powerdevilrc"; cat ~/.config/powerdevilrc; echo "== kscreenlockerrc"; cat ~/.config/kscreenlockerrc; } > /tmp/pd.txt 2>&1
curl -s --data-binary @/tmp/pd.txt 10.0.2.2:8000/out/pd4.txt
