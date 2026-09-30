#!/usr/bin/env python3
"""Makes guest/speech.wav (7 s of synthetic speech, 16 kHz mono) with the espeak-ng library, for timing and
dictation tests in the VM. Needs libespeak-ng.so.1 on the host. Accuracy on real voices will differ."""
import ctypes, wave, array
lib = ctypes.CDLL("libespeak-ng.so.1")
CB = ctypes.CFUNCTYPE(ctypes.c_int, ctypes.POINTER(ctypes.c_short), ctypes.c_int, ctypes.c_void_p)
samples = array.array("h")
def cb(wav, n, ev):
    if wav and n > 0: samples.extend(wav[i] for i in range(n))
    return 0
cbf = CB(cb)
rate = lib.espeak_Initialize(1, 0, None, 0)   # AUDIO_OUTPUT_SYNCHRONOUS
assert rate > 0, rate
lib.espeak_SetSynthCallback(cbf)
lib.espeak_SetVoiceByName(b"en-us")
text = b"Please open the settings and turn on dark mode. Then send a message to my mother and tell her that I will be home before dinner."
lib.espeak_Synth(text, len(text) + 1, 0, 0, 0, 0x1000, None, None)  # espeakCHARS_AUTO
lib.espeak_Synchronize()
# whisper wants 16 kHz mono; resample by linear interpolation from the library rate
src = samples; ratio = rate / 16000
out = array.array("h", (src[min(int(i * ratio), len(src) - 1)] for i in range(int(len(src) / ratio))))
w = wave.open("guest/speech.wav", "wb"); w.setnchannels(1); w.setsampwidth(2); w.setframerate(16000); w.writeframes(out.tobytes()); w.close()
print(f"rate={rate} seconds={len(out)/16000:.1f}")
