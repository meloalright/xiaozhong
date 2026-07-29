#!/usr/bin/env python3
"""容量测试：先占满 --cap 个进寺名额，再连第 cap+1 个——它应被拒并收到
「擁擠 · 稍後再來」提示。用 openssh + pty（和 drive.py 一样，无需 paramiko）。

  python3 ci/capacity.py --host 127.0.0.1 --port 2222 --key k --cap 2 --expect 擁擠
"""
import argparse
import os
import pty
import select
import signal
import sys
import time


def spawn_ssh(host, port, key):
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


def read_for(fd, seconds):
    buf = bytearray()
    end = time.time() + seconds
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.2)
        if r:
            try:
                d = os.read(fd, 4096)
            except OSError:
                break
            if not d:
                break
            buf += d
    return buf.decode("utf-8", "replace")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", required=True)
    ap.add_argument("--port", default="2222")
    ap.add_argument("--key", required=True)
    ap.add_argument("--cap", type=int, required=True, help="进寺名额上限")
    ap.add_argument("--expect", default="擁擠", help="满员时应收到的提示")
    args = ap.parse_args()

    holders = []
    for i in range(args.cap):
        pid, fd = spawn_ssh(args.host, args.port, args.key)
        holders.append((pid, fd))
        read_for(fd, 1.0)  # 让它握手、进寺、握住名额
    time.sleep(2)          # 再等等，确保 cap 个都进寺了
    print(f"已占满 {args.cap} 个名额，试连第 {args.cap + 1} 个…", flush=True)

    pid, fd = spawn_ssh(args.host, args.port, args.key)
    out = read_for(fd, 8.0)
    print("=== 第 N+1 个连接收到 ===", flush=True)
    print(out.replace("\r", "").strip(), flush=True)

    for p, f in holders + [(pid, fd)]:
        try:
            os.close(f)
        except OSError:
            pass
        try:
            os.kill(p, signal.SIGKILL)
        except ProcessLookupError:
            pass

    if args.expect in out:
        print(f"✅ 满员被拒，收到「{args.expect}」提示", flush=True)
        return 0
    print(f"❌ 未收到「{args.expect}」提示", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
