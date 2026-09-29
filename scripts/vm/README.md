# VM test rig (QEMU + OVMF, driven over QMP)

Installs and boots the Zohara ISO in a UEFI virtual machine with an NVMe disk
(named nvme0n1 like the Dell), then lets a script click, type and take
screenshots. This is how the Dell "premature end of file" bug was reproduced
and the fix proven. Dev-machine helper only: nothing here ships in the OS.

Needs: qemu-system-x86 (with KVM), ovmf, python3. Copy this folder to /root/vmtest
(or set D= in the scripts) first.

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
