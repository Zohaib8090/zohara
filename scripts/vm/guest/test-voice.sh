#!/bin/bash
# Tests zohara-voice in the VM: optional package install, both engines directly, engine selection, and the
# whole `zohara-settings --dictate` path with the recording played into a virtual microphone.
# usage (after `sudo -v`):  curl -s 10.0.2.2:8000/test-voice.sh | bash -s LABEL [PACKAGE_FILE]
# PACKAGE_FILE, if given, is fetched from the server (put it in guest/) and installed with pacman -U first.
L=${1:-run}; V=/usr/lib/zohara/whisper; M=/usr/share/zohara/dictation/ggml-base-q8_0.bin
flags=""; for f in avx avx2 bmi2 fma f16c; do grep -qw -m1 $f /proc/cpuinfo && flags="$flags $f"; done
{
echo "=== $L ==="; echo "cpu: $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2)  cores=$(nproc)"; echo "flags:$flags"
if [ -n "$2" ]; then
  echo "before: $(pacman -Q zohara-voice)"
  curl -s "10.0.2.2:8000/$2" -o /tmp/zv.pkg.tar.zst
  sudo pacman -U --noconfirm /tmp/zv.pkg.tar.zst > /tmp/pacman-U.log 2>&1; echo "pacman -U exit=$?"; grep -E 'upgrading|installing|error' /tmp/pacman-U.log | head -5
fi
echo "installed: $(pacman -Q zohara-voice)"
echo "integrity: $(pacman -Qkk zohara-voice 2>&1 | tail -1)"
echo "engines:"; sha256sum $V/* | cut -c1-12,65- | sed 's/^/  /'
echo "uinput: module=$(lsmod | grep -c '^uinput') rule=$(ls /usr/lib/udev/rules.d/70-zohara-uinput.rules 2>&1 | tail -1)"
curl -s 10.0.2.2:8000/speech.wav -o /tmp/s.wav
for B in zohara-whisper zohara-whisper-avx2; do
  s=$(date +%s%N); $V/$B -m $M -f /tmp/s.wav -nt -l en -t 2 > /tmp/o.txt 2>/tmp/e.txt; rc=$?; e=$(date +%s%N)
  echo "direct $B: exit=$rc wall=$(( (e-s)/1000000 ))ms text=[$(tr -s ' \n' ' ' < /tmp/o.txt | cut -c1-40)]"
done
case "$flags" in *avx*avx2*bmi2*fma*f16c*) want=zohara-whisper-avx2;; *) want=zohara-whisper;; esac
echo "settings should pick: $want"
# full dictation: play the recording into a virtual sink that dictation records from
pactl load-module module-null-sink sink_name=zsink >/dev/null 2>&1; pactl set-default-source zsink.monitor 2>&1
systemctl --user stop ydotool.service 2>/dev/null   # no typing: leave the result on the clipboard so it can be read
wl-copy "" 2>/dev/null
( zohara-settings --dictate > /tmp/dict.log 2>&1 & )
sleep 3; paplay -d zsink /tmp/s.wav &
first=""; bin=""
for i in $(seq 1 900); do
  a=$(pgrep -af 'lib/zohara/whisper' | head -1)
  if [ -n "$a" ]; then [ -z "$first" ] && { first=$(date +%s%N); bin=$(echo "$a" | awk '{print $2}'); }; last=$(date +%s%N)
  elif [ -n "$first" ]; then break; fi
  sleep 0.1
done
sleep 2
if [ -n "$first" ]; then echo "dictation ran: $(basename $bin) for $(( (last-first)/1000000 ))ms"; else echo "dictation: engine never started"; fi
echo "dictation output (clipboard): [$(wl-paste 2>/dev/null | cut -c1-140)]"
echo "dictate log: $(grep -E 'recognised|ERROR|WARN' /tmp/dict.log | sed 's/.*dictation: //' | tr '\n' '|' | cut -c1-200)"
pkill -f 'zohara-settings --dictate' 2>/dev/null
} > /tmp/tv-$L.txt 2>&1
curl -s --data-binary @/tmp/tv-$L.txt 10.0.2.2:8000/out/tv-$L.txt >/dev/null && echo "posted $L"
