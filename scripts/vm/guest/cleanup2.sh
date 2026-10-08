#!/bin/bash
{ echo "old.bin: $(ls ~/.cache/zs-test/sub/old.bin 2>&1)"; echo "fresh.bin: $(ls -l ~/.cache/zs-test/fresh.bin 2>&1)"; echo "empty sub dir kept?: $(ls -d ~/.cache/zs-test/sub 2>&1)"; paccache -dk1; paccache -duk0; ls /var/cache/pacman/pkg | wc -l; du -sh /var/cache/pacman/pkg 2>/dev/null; } > /tmp/cl2.txt 2>&1
curl -s --data-binary @/tmp/cl2.txt 10.0.2.2:8000/out/cl-after.txt
