#!/usr/bin/env bash
set -uo pipefail
pass=0; fail=0
ok()  { pass=$((pass+1)); echo "ok   - $1"; }
bad() { fail=$((fail+1)); echo "FAIL - $1"; }
expect_ok()   { local d="$1"; shift; if "$@" >/tmp/out 2>&1; then ok "$d"; else bad "$d"; tail -6 /tmp/out | sed 's/^/       /'; fi; }
expect_fail() { local d="$1"; shift; if "$@" >/tmp/out 2>&1; then bad "$d (should have been refused)"; else ok "$d"; fi; }

pacman -Sy --noconfirm --needed base-devel gnupg >/dev/null 2>&1
id b >/dev/null 2>&1 || useradd -m b
build() { # DIR  -> builds the package in DIR (copied, owned by b) and leaves the file in /out
  local d; d="$(mktemp -d)"; cp -r "$1"/. "$d"/; chown -R b "$d"; mkdir -p /out
  ( cd "$d" && su b -c "PKGDEST=/out makepkg -f --nodeps" >/dev/null 2>&1 ) || { echo "makepkg failed in $1"; return 1; }
}
mkdir -p /out; chmod 777 /out
build /keyring && ls /out/zohara-keyring-*.pkg.tar.zst >/dev/null && ok "zohara-keyring builds" || { bad "zohara-keyring builds"; exit 1; }

# two tiny test packages, in a signed and an unsigned repository
mkdir -p /srv/signed /srv/unsigned
for n in ztest-a ztest-b; do
  d=$(mktemp -d); chown b "$d"
  printf 'pkgname=%s\npkgver=1\npkgrel=1\npkgdesc=t\narch=(any)\nlicense=(MIT)\npackage() { install -Dm644 /dev/null "$pkgdir/usr/share/%s/ok"; }\n' "$n" "$n" > "$d/PKGBUILD"
  ( cd "$d" && su b -c "PKGDEST=$d makepkg -f --nodeps" >/dev/null 2>&1 ) && cp "$d"/*.pkg.tar.zst /srv/signed/ && cp "$d"/*.pkg.tar.zst /srv/unsigned/
done
ZOHARA_PKG_SIGNING_KEY="$(cat /secret.asc)" bash /pkgs/scripts/build-repo.sh /srv/signed stable >/tmp/out 2>&1 && ok "build-repo signs with the real, pinned key" || { bad "build-repo signs with the real key"; tail -5 /tmp/out; }
env -u ZOHARA_PKG_SIGNING_KEY bash /pkgs/scripts/build-repo.sh /srv/unsigned stable >/dev/null 2>&1

# a fresh pacman keyring, as on a newly installed machine
pacman-key --init >/dev/null 2>&1
cat > /tmp/strict.conf <<'C'
[options]
HoldPkg = pacman glibc
Architecture = auto
SigLevel = Required DatabaseOptional
[zohara-stable]
SigLevel = Required DatabaseRequired
Server = file:///srv/signed
C
sed 's#/srv/signed#/srv/unsigned#' /tmp/strict.conf > /tmp/strict-unsigned.conf
printf '[options]\nArchitecture = auto\nSigLevel = Never\n' > /tmp/never.conf
P="pacman --noconfirm --config /tmp/strict.conf"

expect_fail "1. a machine without the key refuses the signed repository" $P -Sy
expect_ok   "2. zohara-keyring installs (adds and trusts the key)"       pacman --noconfirm --config /tmp/never.conf -U /out/zohara-keyring-*.pkg.tar.zst
[ -n "$(pacman-key --list-keys A08F86264C0C384F567697A949D285AF8166C18D 2>/dev/null)" ] && ok "the key is now in the pacman keyring" || bad "the key is now in the pacman keyring"
expect_ok   "3. the same machine now accepts the signed repository"     $P -Sy
expect_ok   "3. ...and installs a package from it"                      $P -S ztest-a
expect_fail "3. ...and still refuses an unsigned repository"            pacman --noconfirm --config /tmp/strict-unsigned.conf -Sy

echo; echo "passed $pass, failed $fail"; [ "$fail" = 0 ]
