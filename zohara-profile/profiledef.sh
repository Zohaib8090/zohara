#!/usr/bin/env bash
# shellcheck disable=SC2034

iso_name="zohara-os"
iso_label="ZOHARA_$(date +%Y%m)"
iso_publisher="Zohara OS <https://github.com/zohara-os>"
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
file_permissions=(
  ["/usr/bin/zohara-deb-engine"]="0:0:755"
  ["/usr/local/bin/zohara-setup-desktop"]="0:0:755"
)