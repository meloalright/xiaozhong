#!/usr/bin/env python3
"""驱动一个 SSH 小钟寺会话：进寺、走动、烧香、撞钟、离开，并校验输出。

用一个真实 PTY 连上 ssh 客户端，按脚本发方向键/空格，把整段输出攒起来，
最后检查每个 --expect 子串是否出现。任何一个缺失就以非零码退出。

  python3 ci/drive.py --host 127.0.0.1 --port 2222 --key ck \
      --steps up,space,enter,left,up,up,up,up,up,up,up,up,space,enter,left,left,left,left \
      --expect 香爐在前 --expect 位燒香 --expect 鐘在側 --expect 位撞鐘 --expect 鐘聲遠去
"""
import argparse
import os
import pty
import select
import sys
import time

KEYMAP = {
    "up": b"\x1b[A",
    "down": b"\x1b[B",
    "left": b"\x1b[D",
    "right": b"\x1b[C",
    "space": b" ",
    "enter": b"\r",
    "q": b"q",
}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", required=True)
    ap.add_argument("--port", default="22")
    ap.add_argument("--key", required=True, help="ssh 私钥文件")
    ap.add_argument("--steps", required=True, help="逗号分隔: up,space,enter,left,...")
    ap.add_argument("--expect", action="append", default=[], help="输出须包含(可多次)")
    ap.add_argument("--settle", type=float, default=0.8, help="每步后读取秒数")
    args = ap.parse_args()

    steps = [s for s in args.steps.split(",") if s]
    for s in steps:
        if s not in KEYMAP:
            print(f"未知按键: {s}", file=sys.stderr)
            return 2

    ssh = [
        "ssh", "-tt", "-p", args.port, "-i", args.key,
        "-o", "IdentitiesOnly=yes",
        "-o", "StrictHostKeyChecking=no",
        "-o", "UserKnownHostsFile=/dev/null",
        "-o", "LogLevel=ERROR",
        "-o", "ConnectTimeout=25",
        "-o", "ServerAliveInterval=5",
        args.host,
    ]

    pid, fd = pty.fork()
    if pid == 0:
        os.execvp("ssh", ssh)
        os._exit(127)

    buf = bytearray()

    def drain(seconds: float) -> bool:
        """读 seconds 秒；对端关闭返回 False。"""
        end = time.time() + seconds
        while time.time() < end:
            r, _, _ = select.select([fd], [], [], 0.1)
            if r:
                try:
                    chunk = os.read(fd, 65536)
                except OSError:
                    return False
                if not chunk:
                    return False
                buf.extend(chunk)
        return True

    drain(3.0)  # 首帧 / 横幅（生产上冷启动可能略慢）
    # 首次带这把公钥进寺会先出现选头像界面，按 Enter 确认
    if "先擇一副面容" in buf.decode("utf-8", "ignore"):
        os.write(fd, KEYMAP["enter"])
        drain(1.2)

    for s in steps:
        os.write(fd, KEYMAP[s])
        if not drain(args.settle):
            break
    drain(1.5)  # 收尾：离寺帧 / 连接关闭

    try:
        os.close(fd)
    except OSError:
        pass
    try:
        os.waitpid(pid, 0)
    except OSError:
        pass

    out = buf.decode("utf-8", "ignore")
    tail = out[-1800:].replace("\x1b", "^[")
    print("---- captured tail ----")
    print(tail)
    print("-----------------------")

    missing = [e for e in args.expect if e not in out]
    if missing:
        print("FAIL — 缺失:", missing)
        return 1
    print("OK — 全部命中:", args.expect)
    return 0


if __name__ == "__main__":
    sys.exit(main())
