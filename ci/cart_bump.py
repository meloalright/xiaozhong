#!/usr/bin/env python3
"""推车撞「人」演示：先接一个占位者（挑好头像、占住一格不动），再接推车者按步骤推，
把推车者最后一帧打进日志。位置靠 XIAOZHONGSI_FAKE_CART + 落脚点的确定顺序控制。

  python3 ci/cart_bump.py --host 127.0.0.1 --port 2222 \
      --holder-key /tmp/hk --pusher-key /tmp/pk \
      --pusher-steps down,right,up --expect 購物車在側
"""
import argparse
import os
import pty
import re
import select
import signal
import sys
import time

KEYMAP = {
    "up": b"\x1b[A",
    "down": b"\x1b[B",
    "left": b"\x1b[D",
    "right": b"\x1b[C",
    "space": b" ",
    "enter": b"\r",
}
ANSI = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]")


def clean(raw):
    frame = raw.split("\x1b[2J")[-1]
    frame = ANSI.sub("", frame).replace("\r", "")
    lines = [ln.rstrip() for ln in frame.split("\n")]
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip():
        lines.pop()
    return "\n".join(lines)


def spawn(host, port, key):
    ssh = [
        "ssh", "-tt", "-p", port, "-i", key,
        "-o", "IdentitiesOnly=yes",
        "-o", "StrictHostKeyChecking=no",
        "-o", "UserKnownHostsFile=/dev/null",
        "-o", "LogLevel=ERROR",
        "-o", "ConnectTimeout=25",
        host,
    ]
    pid, fd = pty.fork()
    if pid == 0:
        os.execvp("ssh", ssh)
        os._exit(127)
    return pid, fd


def drain(fd, sec):
    buf = bytearray()
    end = time.time() + sec
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.2)
        if r:
            try:
                d = os.read(fd, 65536)
            except OSError:
                break
            if not d:
                break
            buf += d
    return buf


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", required=True)
    ap.add_argument("--port", default="2222")
    ap.add_argument("--holder-key", required=True)
    ap.add_argument("--pusher-key", required=True)
    ap.add_argument("--pusher-steps", required=True)
    ap.add_argument("--expect", default="購物車在側")
    a = ap.parse_args()

    procs = []
    # 占位者：连、挑头像、占住落脚点不动
    hp, hf = spawn(a.host, a.port, a.holder_key)
    procs.append((hp, hf))
    drain(hf, 3.0)
    os.write(hf, KEYMAP["enter"])
    drain(hf, 1.5)

    # 推车者：连、挑头像、按步骤推，全程收进 buf
    pp, pf = spawn(a.host, a.port, a.pusher_key)
    procs.append((pp, pf))
    buf = bytearray()
    buf += drain(pf, 3.0)
    os.write(pf, KEYMAP["enter"])
    buf += drain(pf, 1.5)
    for s in a.pusher_steps.split(","):
        os.write(pf, KEYMAP[s])
        buf += drain(pf, 0.5)

    out = buf.decode("utf-8", "ignore")
    print(clean(out), flush=True)

    for p, f in procs:
        try:
            os.close(f)
        except OSError:
            pass
        try:
            os.kill(p, signal.SIGKILL)
        except ProcessLookupError:
            pass

    if a.expect in out:
        print(f"✅ 命中「{a.expect}」", flush=True)
        return 0
    print(f"❌ 未命中「{a.expect}」", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
