#!/bin/bash
# Boots an already-installed disk image with no ISO attached (tests the real boot path).
#   run-disk.sh [DISK] [FORMAT]     DISK defaults to $ZOHARA_VM_DIR/disk.qcow2 (/root/vmtest if unset), FORMAT to qcow2
D="${ZOHARA_VM_DIR:-/root/vmtest}"
exec qemu-system-x86_64 -name zohara-installed -enable-kvm -cpu host -smp 4 -m 4096 -machine q35 \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=$D/OVMF_VARS.fd \
  -drive file=${1:-$D/disk.qcow2},if=none,id=nvm,format=${2:-qcow2} \
  -device nvme,serial=zohara0001,drive=nvm,bootindex=1 \
  -device VGA,xres=1280,yres=800 -device qemu-xhci,id=xhci -device usb-tablet -device usb-kbd \
  -nic user,model=virtio-net-pci -display none \
  -qmp unix:$D/qmp.sock,server=on,wait=off -serial file:$D/serial-disk.log
