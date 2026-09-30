#!/bin/bash
# Boots the installed disk with a restricted virtual CPU, for old-hardware tests.
#   run-cpu.sh [CPU_MODEL] [CORES]      e.g.  run-cpu.sh qemu64 2   |   Nehalem 2   |   Haswell 2   |   host 4
# qemu64 = basic x86-64 (no AVX), Nehalem = 2008 (no AVX), Haswell = 2013 (AVX2). `qemu-system-x86_64 -cpu help`
# lists the rest. The guest sees only those instructions; the host's clock speed is unchanged, so timings
# understate a genuinely old chip. Uses $ZOHARA_VM_DIR (default /root/vmtest) like the other scripts.
D="${ZOHARA_VM_DIR:-/root/vmtest}"
exec qemu-system-x86_64 -name zohara-cpu -enable-kvm -cpu "${1:-host}" -smp "${2:-4}" -m 4096 -machine q35 \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=$D/OVMF_VARS.fd \
  -drive file=$D/disk.qcow2,if=none,id=nvm,format=qcow2 \
  -device nvme,serial=zohara0001,drive=nvm,bootindex=1 \
  -device VGA,xres=1280,yres=800 -device qemu-xhci,id=xhci -device usb-tablet -device usb-kbd \
  -nic user,model=virtio-net-pci -display none \
  -qmp unix:$D/qmp.sock,server=on,wait=off -serial file:$D/serial-cpu.log
