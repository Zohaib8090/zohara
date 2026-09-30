#!/usr/bin/env python3
"""Dev-machine helper: hands scripts to the test VM and collects what the VM sends back.

  python3 srv.py        serves guest/ on 127.0.0.1:8000; POSTs to /out/NAME are saved as out/NAME

QEMU's user networking makes the host's loopback reachable from the guest as 10.0.2.2, so in the guest:
  curl -s 10.0.2.2:8000/state.sh | bash -s LABEL
and the script posts its result with:  curl -s --data-binary @FILE 10.0.2.2:8000/out/NAME
That avoids typing long commands into the VM. Listens on loopback only. Nothing here ships in the OS.
"""
import http.server
import os
import re

ROOT = os.path.dirname(os.path.abspath(__file__))
os.makedirs(os.path.join(ROOT, "out"), exist_ok=True)


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **k):
        super().__init__(*a, directory=os.path.join(ROOT, "guest"), **k)

    def do_POST(self):
        name = re.sub(r"[^A-Za-z0-9._-]", "_", self.path.rsplit("/", 1)[-1]) or "out"
        body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        with open(os.path.join(ROOT, "out", name), "wb") as f:
            f.write(body)
        self.send_response(200)
        self.end_headers()

    def log_message(self, *a):
        pass


if __name__ == "__main__":
    http.server.ThreadingHTTPServer(("127.0.0.1", 8000), Handler).serve_forever()
