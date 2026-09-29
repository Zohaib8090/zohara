#!/bin/bash
# Starts the Zohara test VM: UEFI, NVMe disk (named nvme0n1 like the Dell), KVM.
#   run-vm.sh ISO [DISK]     (DISK defaults to /root/vmtest/disk.qcow2)
# Screen and keyboard are driven through /root/vmtest/qmp.sock by vm.py.
ISO="${1:?usage: run-vm.sh ISO [DISK]}"
DISK="${2:-/root/vmtest/disk.qcow2}"
D=/root/vmtest

exec qemu-system-x86_64 \
    -name zohara-test \
    -enable-kvm -cpu host -smp 4 -m 4096 -machine q35 \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
    -drive if=pflash,format=raw,file=$D/OVMF_VARS.fd \
    -drive file="$DISK",if=none,id=nvm,format=qcow2 \
    -device nvme,serial=zohara0001,drive=nvm,bootindex=2 \
    -drive file="$ISO",media=cdrom,readonly=on,if=none,id=cd0 \
    -device ide-cd,drive=cd0,bootindex=1 \
    -device VGA,xres=1280,yres=800 \
    -device qemu-xhci,id=xhci -device usb-tablet -device usb-kbd \
    -nic user,model=virtio-net-pci \
    -display none \
    -qmp unix:$D/qmp.sock,server=on,wait=off \
    -serial file:$D/serial.log
