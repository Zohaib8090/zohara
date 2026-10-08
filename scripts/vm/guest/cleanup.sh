#!/bin/bash
# Test setup for Storage > Clean up: an old 20 MB cache file, a fresh 3 MB one, an old package file nobody needs.
mkdir -p ~/.cache/zs-test/sub
head -c 20000000 /dev/zero > ~/.cache/zs-test/sub/old.bin; touch -d '5 days ago' ~/.cache/zs-test/sub/old.bin
head -c 3000000 /dev/zero > ~/.cache/zs-test/fresh.bin
{ echo "settings: $(pacman -Q zohara-settings)"; paccache -dk1; paccache -duk0; du -sh ~/.cache /var/cache/pacman/pkg; } > /tmp/cl.txt 2>&1
curl -s --data-binary @/tmp/cl.txt 10.0.2.2:8000/out/cl-before.txt
pkill zohara-settings; nohup zohara-settings --page Storage >/dev/null 2>&1 &
