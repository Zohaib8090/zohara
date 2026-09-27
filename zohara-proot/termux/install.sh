#!/data/data/com.termux/files/usr/bin/bash
# Installs Zohara OS in Termux. Run:
#   curl -fsSL https://github.com/Zohaib8090/zohara/releases/download/proot-latest/install.sh | bash
set -e
BASE="https://github.com/Zohaib8090/zohara/releases/download/proot-latest"
ARCHIVE="zohara-rootfs-aarch64.tar.xz"
DL="$HOME/.zohara-download"

[ "$(uname -m)" = aarch64 ] || { echo "Zohara for phones needs a 64-bit ARM phone (aarch64)." >&2; exit 1; }

echo "Installing what Zohara needs in Termux…"
pkg update -y
pkg install -y x11-repo
pkg install -y proot-distro termux-x11-nightly pulseaudio curl
# Optional: lets `zohara gpu` use the phone's GPU.
pkg install -y virglrenderer-android || true

curl -fsSL "$BASE/zohara" -o "$PREFIX/bin/zohara"
chmod 755 "$PREFIX/bin/zohara"

if proot-distro list 2>/dev/null | grep -q '^ *zohara\b\|Alias: *zohara'; then
    echo "Zohara is already installed. To reinstall it first run:  proot-distro remove zohara"
    exit 0
fi

# proot-distro 5 installs from an archive file (or a Docker image), so the
# system is downloaded first and checked against its published checksum.
mkdir -p "$DL"
echo "Downloading Zohara OS (about 870 MB). If it stops, run this command again: it resumes."
curl -fL --retry 5 -C - -o "$DL/$ARCHIVE" "$BASE/$ARCHIVE"
curl -fsSL "$BASE/$ARCHIVE.sha256" -o "$DL/$ARCHIVE.sha256"
echo "Checking the download…"
(cd "$DL" && sha256sum -c "$ARCHIVE.sha256") || { echo "The download is damaged. Run the command again." >&2; rm -f "$DL/$ARCHIVE"; exit 1; }

echo "Installing Zohara OS (a few minutes)…"
if proot-distro install --name zohara "$DL/$ARCHIVE"; then
    rm -rf "$DL"
else
    # proot-distro 4 and older take a definition file instead of an archive.
    curl -fsSL "$BASE/zohara.sh" -o "$PREFIX/etc/proot-distro/zohara.sh"
    proot-distro install zohara
    rm -rf "$DL"
fi

cat <<'EOF'

Zohara OS is installed.

One more thing: install the Termux:X11 app, which shows the desktop:
  https://github.com/termux/termux-x11/releases  (the "app-arm64-v8a-debug.apk")

Then start Zohara any time by typing:  zohara
EOF
