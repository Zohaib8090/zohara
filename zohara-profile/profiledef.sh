#!/usr/bin/env bash
# shellcheck disable=SC2034

iso_name="zohara-os"
iso_label="ZOHARA_ISO"
iso_publisher="Zohara OS <https://github.com/Zohaib8090/zohara>"
iso_application="Zohara OS Live/Install CD"
iso_version="$(date +%Y.%m.%d)"
install_dir="arch"
buildmodes=('iso')
bootmodes=('bios.syslinux' 'uefi.systemd-boot')
arch="x86_64"
pacman_conf="pacman.conf"
airootfs_image_type="squashfs"
airootfs_image_tool_options=('-comp' 'zstd' '-b' '1M')
efiboot_efi_img_size='512M'
airootfs_script="/root/customize_airootfs.sh"

file_permissions=(
  ["/root/customize_airootfs.sh"]="0:0:755"
  ["/usr/bin/zohara-deb-engine"]="0:0:755"
  ["/usr/bin/zohara-settings"]="0:0:755"
  ["/usr/bin/zohara-store"]="0:0:755"
  # NOTE: there is deliberately no entry for /usr/local/bin/customize_airootfs.sh. That file was a
  # stale Aug-5 copy of the chroot script (Plymouth watermark + pixmap only -- a strict subset of
  # what /root/customize_airootfs.sh already does), invoked by nothing, yet installed 0755 into every
  # user's PATH by both the ISO and the OTA zohara-system package. It was removed on 2026-08-24.
  # Do not re-add it: airootfs_script below is the only copy mkarchiso runs, and mkarchiso deletes
  # /root/customize_airootfs.sh from the image afterwards precisely so it does not ship.
  ["/usr/local/bin/zohara-setup-desktop"]="0:0:755"
  ["/usr/local/bin/brave-origin"]="0:0:755"
  ["/usr/local/bin/zohara-welcome"]="0:0:755"
  ["/usr/local/bin/zohara-migrate"]="0:0:755"
  ["/usr/local/bin/zohara-install-kernel"]="0:0:755"
  ["/usr/local/bin/zohara-usermgr"]="0:0:755"
)