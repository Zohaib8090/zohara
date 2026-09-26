#!/data/data/com.termux/files/usr/bin/bash
# Installs Zohara OS in Termux. Run:
#   curl -fsSL https://github.com/Zohaib8090/zohara/releases/download/proot-latest/install.sh | bash
set -e
BASE="https://github.com/Zohaib8090/zohara/releases/download/proot-latest"

[ "$(uname -m)" = aarch64 ] || { echo "Zohara for phones needs a 64-bit ARM phone (aarch64)." >&2; exit 1; }

echo "Installing what Zohara needs in Termux…"
pkg update -y
pkg install -y x11-repo
pkg install -y proot-distro termux-x11-nightly pulseaudio curl

mkdir -p "$PREFIX/etc/proot-distro"
curl -fsSL "$BASE/zohara.sh" -o "$PREFIX/etc/proot-distro/zohara.sh"
curl -fsSL "$BASE/zohara" -o "$PREFIX/bin/zohara"
chmod 755 "$PREFIX/bin/zohara"

echo "Downloading and installing Zohara OS (about 1 GB)…"
proot-distro install zohara

cat <<'EOF'

Zohara OS is installed.

One more thing: install the Termux:X11 app, which shows the desktop:
  https://github.com/termux/termux-x11/releases  (the "app-arm64-v8a-debug.apk")

Then start Zohara any time by typing:  zohara
EOF
