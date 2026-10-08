#!/bin/bash
# Test setup for web apps: a second "browser" (a stub that logs its arguments, then runs brave-origin), then Settings.
sudo tee /usr/local/bin/vivaldi-stable >/dev/null <<'STUB'
#!/bin/sh
echo "$@" >> /tmp/vivaldi-args.txt
exec /usr/local/bin/brave-origin "$@"
STUB
sudo chmod +x /usr/local/bin/vivaldi-stable
pkill zohara-settings
nohup zohara-settings --page Apps >/dev/null 2>&1 &
