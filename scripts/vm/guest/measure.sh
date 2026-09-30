#!/bin/bash
# usage: curl -s 10.0.2.2:8000/measure.sh | bash -s LABEL
L=${1:-run}; M=/usr/share/zohara/dictation/ggml-base-q8_0.bin; D=/usr/lib/zohara/whisper
curl -s 10.0.2.2:8000/speech.wav -o /tmp/s.wav
{
echo "=== $L ==="
echo "cpu: $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2)"
echo "flags: $(for f in sse4_2 avx avx2 fma f16c; do grep -qw -m1 $f /proc/cpuinfo && printf '%s ' $f; done)"
echo "cores: $(nproc)  $(free -m | awk 'NR==2{print "ram_mb="$2" free_mb="$7}')"
ls -l $M | awk '{print "model bytes:", $5}'
for B in zohara-whisper zohara-whisper-avx2; do
  for i in 1 2; do
    s=$(date +%s.%N)
    $D/$B -m $M -f /tmp/s.wav -nt -l en > /tmp/out.txt 2> /tmp/err.txt; rc=$?
    e=$(date +%s.%N)
    printf '%s run%s: exit=%s wall=%.1fs\n' $B $i $rc $(echo "$e - $s" | bc -l 2>/dev/null || python3 -c "print($e-$s)")
  done
  grep -E 'load time|encode time|total time|system_info' /tmp/err.txt | sed 's/^/    /' | cut -c1-600
  echo "    text: $(tr -s ' \n' ' ' < /tmp/out.txt | cut -c1-600)"
done
} > /tmp/m-$L.txt 2>&1
curl -s --data-binary @/tmp/m-$L.txt 10.0.2.2:8000/out/m-$L.txt > /dev/null && echo "posted $L"
