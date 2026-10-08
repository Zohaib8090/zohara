#!/bin/bash
{ echo "== args seen by the vivaldi stub"; cat /tmp/vivaldi-args.txt; echo "== desktop file"; cat ~/.local/share/applications/zohara-webapp-*.desktop; echo "== files"; ls -l ~/.local/share/zohara-webapps/*/; file ~/.local/share/zohara-webapps/*/icon.*; } > /tmp/w.txt 2>&1
curl -s --data-binary @/tmp/w.txt 10.0.2.2:8000/out/w.txt
