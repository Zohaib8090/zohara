# VM test rig (QEMU + OVMF, driven over QMP)

Installs and boots the Zohara ISO in a UEFI virtual machine with an NVMe disk
(named nvme0n1 like the Dell), then lets a script click, type and take
screenshots. This is how the Dell "premature end of file" bug was reproduced
and the fix proven. Dev-machine helper only: nothing here ships in the OS.

Needs: qemu-system-x86 (with KVM), ovmf, python3. Working dir: see "Running as a normal user" below (default /root/vmtest).

    cp /usr/share/OVMF/OVMF_VARS_4M.fd /root/vmtest/OVMF_VARS.fd
    qemu-img create -f qcow2 /root/vmtest/disk.qcow2 40G
    ./run-vm.sh path/to/zohara.iso /root/vmtest/disk.qcow2 &   # install
    python3 vm.py shot s.png                                    # look at the screen
    python3 vm.py click 640 414                                 # click (pixels of the shot)
    python3 vm.py type "text"; python3 vm.py key ret
    python3 vm.py quit
    ./run-disk.sh /root/vmtest/disk.qcow2 &                     # boot the result, no ISO

Test a second USB stick: see docs/HANDOFF-2026-09-29.md ("Testing the problem-report button").

Tips learned the hard way
- Installer window is taller than 800px: double-click its title bar to maximise.
- The live session locks its screen mid-install: move the pointer every 30s.
- Type slowly and clear each field first; fast typing drops keys.
- On WSL, launch detached with `setsid nohup ... & disown` AND keep the call alive with a sleep.

## Running as a normal user (no root, no sfdisk)

All the scripts read `ZOHARA_VM_DIR` (default `/root/vmtest`, so nothing changes for the old setup):

    export ZOHARA_VM_DIR=$HOME/zohara-vm; mkdir -p $ZOHARA_VM_DIR
    cp /usr/share/OVMF/OVMF_VARS_4M.fd $ZOHARA_VM_DIR/OVMF_VARS.fd
    qemu-img create -f qcow2 $ZOHARA_VM_DIR/disk.qcow2 40G
    ./run-vm.sh path/to/zohara.iso &      # then drive it with:  python3 vm.py shot s.png  etc.

Needs `/dev/kvm` access (a normal user is fine). Keep a copy of the installed disk (`cp --sparse=always
disk.qcow2 disk-before.qcow2`) so a test can be repeated from a clean state.

`vm.py` types slowly on purpose (120 ms key hold, 0.15 s between keys). At the old speed a key could stick in
the guest and it stopped answering (see `GOTCHA.md`).

## Long commands: serve a script from the host instead of typing it

`srv.py` serves `guest/` on the host's loopback and saves what the guest posts into `out/` (git-ignored).
QEMU's user networking makes the host `10.0.2.2` from inside the guest. Start it with `python3 srv.py &`,
open a terminal in the guest (`python3 vm.py key ctrl-alt-t`), and type one short line. Results appear in
`out/` (`LABEL.txt` for `state.sh`, `m-LABEL.txt` for `measure.sh`, `tv-LABEL.txt` for `test-voice.sh`):

    curl -s 10.0.2.2:8000/state.sh | bash -s before

| Script in `guest/` | What it does |
|---|---|
| `state.sh LABEL` | Records the boot state: kernel, `/boot` files, sparse-hole check, initramfs, pin, DKMS |
| `upgrade.sh DATE` | Moves the pinned Arch date forward (what Zohara Store does) and runs the upgrade. Run with `sudo` |
| `test-update.sh` | A real `pacman -Sy` / `-Su` against the live `zohara-packages` release |
| `measure.sh LABEL` | Times both whisper engines on `guest/speech.wav` |
| `test-voice.sh LABEL [PKG]` | Optional package install, both engines, engine selection, and full `zohara-settings --dictate` with the recording played into a virtual microphone |

`mkspeech.py` makes `guest/speech.wav` (synthetic speech, needs the host's espeak-ng library). Use `sudo` in the
guest once with `echo PASSWORD | sudo -S -v` typed by hand; never put real passwords in a script you commit.
To test a package you built, put the `.pkg.tar.zst` in `guest/` and pass its file name to `test-voice.sh`.

## Restricting the virtual CPU (old-hardware tests)

`run-cpu.sh CPU CORES` boots the installed disk as `qemu64` (basic x86-64, no AVX), `Nehalem` (2008, no AVX),
`Haswell` (2013, AVX2) or `host`. The guest sees only those instructions; the host's clock is unchanged, so
timings understate a genuinely old chip. For pinned cores when timing in a container, use physical cores
(`lscpu -e`: two CPUs with different CORE numbers), not two hyper-threads of one core.

## Hot-plugging a USB stick without root

Build the image with Python and `mkfs.vfat --offset`, then attach it over QMP (`vm.py` has the helper
`qmp()`): write an MBR with one FAT32 partition at sector 2048 (bytes 446..462 and the `55 aa` signature),
`mkfs.vfat -F 32 -n LABEL --offset 2048 usb.img`, then `blockdev-add` (file + raw) and `device_add`
`usb-storage` on `bus=xhci.0`. To read files back on the host without root: `dd if=usb.img of=part.img
bs=512 skip=2048` then `7z l part.img` and `7z x -so part.img FILE`. For a stick with no partition table,
skip the MBR and run `mkfs.vfat` on the whole file.

## What has been checked with this rig (2026-09-30)

Install, boot of the installed disk, the problem-report button, a `pacman` kernel upgrade with a reboot, a
`pacman -Sy` against the live release, and voice typing on four virtual CPUs. Details and numbers:
`docs/HANDOFF-2026-09-30.md`.
