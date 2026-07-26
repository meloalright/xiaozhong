#!/usr/bin/env python3
"""驱动一个 SSH 小钟寺会话：选头像、走动、烧香、撞钟、离开，并把每一步的画面
打进 CI 日志（用 GitHub Actions 的 ::group:: 折叠成一段段），最后校验输出。

  python3 ci/drive.py --host 127.0.0.1 --port 2222 --key ck --pick 3 \
      --steps up,space,enter,left,up,up,up,up,up,up,up,up,space,enter,left,left,left,left \
      --expect 先擇一副面容 --expect 香爐在前 --expect 位燒香 \
      --expect 鐘在側 --expect 位撞鐘 --expect 鐘聲遠去
"""
import argparse
import os
import pty
import re
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

# 每一步给个可读的中文说明，让日志像一场演出
STEP_NOTE = {
    "up": "上",
    "down": "下",
    "left": "左",
    "right": "右",
    "space": "空格（撞钟/烧香）",
    "enter": "任意键（起身）",
    "q": "q（离寺）",
}

ANSI = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]")


def clean_screen(raw: str) -> str:
    """取最后一帧（最近一次清屏之后），抹掉 ANSI 转义，去掉上下空行。"""
    frame = raw.split("\x1b[2J")[-1]
    frame = ANSI.sub("", frame).replace("\r", "")
    lines = [ln.rstrip() for ln in frame.split("\n")]
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip():
        lines.pop()
    return "\n".join(lines)


def group(title: str) -> None:
    print(f"::group::{title}", flush=True)


def endgroup() -> None:
    print("::endgroup::", flush=True)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", required=True)
    ap.add_argument("--port", default="22")
    ap.add_argument("--key", required=True, help="ssh 私钥文件")
    ap.add_argument("--steps", required=True, help="逗号分隔: up,space,enter,left,...")
    ap.add_argument("--expect", action="append", default=[], help="输出须包含(可多次)")
    ap.add_argument("--settle", type=float, default=0.8, help="每步后读取秒数")
    ap.add_argument("--pick", type=int, default=0, help="选头像时向右挪几格再确认")
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

    def text() -> str:
        return buf.decode("utf-8", "ignore")

    def send(key: str) -> None:
        os.write(fd, KEYMAP[key])

    drain(3.0)  # 首帧 / 横幅（生产上冷启动可能略慢）

    # ── ① 选头像 ──────────────────────────────────────────────
    group("① 進寺門 · 選頭像")
    if "先擇一副面容" in text():
        print(clean_screen(text()))
        for _ in range(args.pick):
            send("right")
            drain(0.4)
        if args.pick:
            print(f"\n  → 向右挪 {args.pick} 格，選了一副新面容")
            print(clean_screen(text()))
        send("enter")
        drain(1.2)
        print("\n  → 確認進寺")
    else:
        print("  這把鑰匙寺裡認得，記著上回的面容，直接進寺")
    print(clean_screen(text()))
    endgroup()

    # ── ② 逐步走位 ────────────────────────────────────────────
    for i, s in enumerate(steps, 1):
        send(s)
        if not drain(args.settle):
            group(f"步 {i}/{len(steps)} · {STEP_NOTE[s]} —— 已離寺，連接關閉")
            print(clean_screen(text()))
            endgroup()
            break
        group(f"步 {i}/{len(steps)} · {STEP_NOTE[s]}")
        print(clean_screen(text()))
        endgroup()

    drain(1.5)

    try:
        os.close(fd)
    except OSError:
        pass
    try:
        os.waitpid(pid, 0)
    except OSError:
        pass

    # ── ③ 校验 ────────────────────────────────────────────────
    out = text()
    missing = [e for e in args.expect if e not in out]
    group("③ 校驗")
    for e in args.expect:
        print(f"  {'✓' if e not in missing else '✗'}  {e}")
    endgroup()

    if missing:
        print(f"FAIL — 缺失: {missing}")
        return 1
    print(f"OK — 全部命中: {args.expect}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
