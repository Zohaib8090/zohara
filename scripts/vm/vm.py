#!/usr/bin/env python3
"""Remote control for the test VM over QEMU's QMP socket.

  vm.py shot FILE.png          screenshot of the VM screen
  vm.py click X Y [double]     left-click at pixel X,Y of the last screenshot size
  vm.py rclick X Y             right-click
  vm.py move X Y               move the pointer
  vm.py type "text"            type text (US layout)
  vm.py key ret|tab|esc|...    press keys, e.g.  key ctrl-alt-f2
  vm.py status                 running / paused
  vm.py quit                   power the VM off

Only used to drive test VMs; nothing here ships in the OS.
"""
import json
import os
import socket
import struct
import sys
import time

VM_DIR = os.environ.get("ZOHARA_VM_DIR", "/root/vmtest")
SOCK = os.path.join(VM_DIR, "qmp.sock")

SHIFT_SYMBOLS = {
    "!": "1", "@": "2", "#": "3", "$": "4", "%": "5", "^": "6", "&": "7",
    "*": "8", "(": "9", ")": "0", "_": "minus", "+": "equal", "{": "bracket_left",
    "}": "bracket_right", "|": "backslash", ":": "semicolon", '"': "apostrophe",
    "<": "comma", ">": "dot", "?": "slash", "~": "grave_accent",
}
PLAIN_SYMBOLS = {
    " ": "spc", "-": "minus", "=": "equal", "[": "bracket_left", "]": "bracket_right",
    "\\": "backslash", ";": "semicolon", "'": "apostrophe", ",": "comma", ".": "dot",
    "/": "slash", "`": "grave_accent", "\n": "ret", "\t": "tab",
}


def qmp(command, arguments=None):
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.settimeout(30)
    s.connect(SOCK)
    f = s.makefile("rw")
    f.readline()  # greeting
    f.write(json.dumps({"execute": "qmp_capabilities"}) + "\n")
    f.flush()
    f.readline()
    msg = {"execute": command}
    if arguments:
        msg["arguments"] = arguments
    f.write(json.dumps(msg) + "\n")
    f.flush()
    while True:
        line = f.readline()
        if not line:
            raise RuntimeError("QMP closed")
        reply = json.loads(line)
        if "return" in reply or "error" in reply:
            s.close()
            if "error" in reply:
                raise RuntimeError(reply["error"])
            return reply["return"]


def png_size(path):
    with open(path, "rb") as fh:
        head = fh.read(24)
    return struct.unpack(">II", head[16:24])


def shot(path):
    qmp("screendump", {"filename": path, "format": "png"})
    time.sleep(0.3)
    w, h = png_size(path)
    open(os.path.join(VM_DIR, "size"), "w").write(f"{w} {h}")
    print(f"{w}x{h}")


def screen_size():
    try:
        w, h = open(os.path.join(VM_DIR, "size")).read().split()
        return int(w), int(h)
    except Exception:
        return 1280, 800


def abs_events(x, y):
    w, h = screen_size()
    ax = int(x * 32767 / max(w - 1, 1))
    ay = int(y * 32767 / max(h - 1, 1))
    return [
        {"type": "abs", "data": {"axis": "x", "value": ax}},
        {"type": "abs", "data": {"axis": "y", "value": ay}},
    ]


def move(x, y):
    qmp("input-send-event", {"events": abs_events(x, y)})


def click(x, y, button="left", double=False):
    move(x, y)
    time.sleep(0.15)
    for _ in range(2 if double else 1):
        qmp("input-send-event", {"events": [{"type": "btn", "data": {"button": button, "down": True}}]})
        time.sleep(0.08)
        qmp("input-send-event", {"events": [{"type": "btn", "data": {"button": button, "down": False}}]})
        time.sleep(0.12)


def press(*codes):
    qmp("send-key", {"keys": [{"type": "qcode", "data": c} for c in codes], "hold-time": 120})


def type_text(text):
    for ch in text:
        if ch.isalpha() and ch.isupper():
            press("shift", ch.lower())
        elif ch.isalpha() or ch.isdigit():
            press(ch)
        elif ch in SHIFT_SYMBOLS:
            press("shift", SHIFT_SYMBOLS[ch])
        elif ch in PLAIN_SYMBOLS:
            press(PLAIN_SYMBOLS[ch])
        else:
            raise SystemExit(f"cannot type {ch!r}")
        time.sleep(0.15)   # slower than this and a key can stick in the guest (see GOTCHA.md)


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return
    cmd, args = sys.argv[1], sys.argv[2:]
    if cmd == "shot":
        shot(args[0])
    elif cmd == "click":
        click(int(args[0]), int(args[1]), double=len(args) > 2 and args[2] == "double")
    elif cmd == "rclick":
        click(int(args[0]), int(args[1]), button="right")
    elif cmd == "move":
        move(int(args[0]), int(args[1]))
    elif cmd == "type":
        type_text(args[0])
    elif cmd == "key":
        for spec in args:
            press(*spec.split("-")) if spec != "-" else press("minus")
            time.sleep(0.1)
    elif cmd == "status":
        print(qmp("query-status"))
    elif cmd == "quit":
        qmp("quit")
    else:
        print(__doc__)


if __name__ == "__main__":
    main()
