#!/usr/bin/env python3
"""Exercise CLI command lifecycles against an isolated, local zenohd.

Uses only Python's standard library. Build default and alloc CLI binaries first;
pass their paths with --cli and --alloc-cli. The script starts/stops its own
router and clients, without changing an existing router or local credentials.
"""

import argparse
import os
import pty
import select
import selectors
import shutil
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path


class Console:
    def __init__(self, binary, endpoint):
        self.process = subprocess.Popen(
            [str(binary)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            env={**os.environ, "ENDPOINT": endpoint, "RUST_LOG": "error"},
        )
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)
        self.output = b""
        self.expect(b"zenoh> ")

    def expect(self, token, since=0, timeout=10):
        deadline = time.monotonic() + timeout
        while token not in self.output[since:]:
            left = deadline - time.monotonic()
            if left <= 0 or not self.selector.select(left):
                raise AssertionError(f"Missing {token!r}; tail={self.output[-2500:]!r}")
            data = os.read(self.process.stdout.fileno(), 8192)
            if not data:
                raise AssertionError(
                    f"CLI exited {self.process.poll()}: {self.output[-2500:]!r}"
                )
            self.output += data
        return self.output[since:]

    def send(self, text):
        mark = len(self.output)
        self.process.stdin.write(text.encode() if isinstance(text, str) else text)
        self.process.stdin.flush()
        return mark

    def command(self, line, expect=b"zenoh> "):
        mark = self.send(line + "\n")
        out = self.expect(expect, mark)
        if any(
            error in out
            for error in (
                b"command error:",
                b"session error:",
                b"connection error:",
                b"broker error:",
                b"cleanup failed",
            )
        ):
            raise AssertionError(out)
        return out

    def cancel(self):
        mark = self.send(b"\x03")
        output = self.expect(b"zenoh> ", mark)
        assert b"session=closed" in self.command("status"), output
        return output

    def close(self):
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
        self.selector.close()
        self.process.stdin.close()
        self.process.stdout.close()


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def check_one_shot(binary, endpoint):
    for arguments, expected in [
        (["--help"], b"z_get"),
        (["help", "z_get"], b"--timeout-ms"),
    ]:
        result = subprocess.run(
            [str(binary), *arguments], capture_output=True, timeout=10
        )
        assert result.returncode == 0 and expected in result.stdout, (
            result.stderr + result.stdout
        )
    result = subprocess.run(
        [str(binary), "z_put", "cli_smoke/once", 'space and quote: "ok"'],
        env={**os.environ, "ENDPOINT": endpoint},
        capture_output=True,
        timeout=10,
    )
    assert result.returncode == 0, result.stderr + result.stdout
    result = subprocess.run([str(binary), "z_get"], capture_output=True, timeout=10)
    assert result.returncode == 2, result.stderr + result.stdout


def check_serial_terminal():
    master, slave = pty.openpty()
    script = Path(__file__).resolve().parents[2] / "common/serial_terminal.py"
    terminal = subprocess.Popen(
        [sys.executable, str(script), os.ttyname(slave)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        assert b"Ctrl-C" in terminal.stderr.readline()
        terminal.stdin.write(b"\x03")
        terminal.stdin.flush()
        assert select.select([master], [], [], 2)[0]
        assert os.read(master, 64) == b"\x03"
        terminal.stdin.write(b"\x1d")
        terminal.stdin.flush()
        assert terminal.wait(2) == 0
    finally:
        if terminal.poll() is None:
            terminal.kill()
            terminal.wait()
        os.close(master)
        os.close(slave)
        terminal.stdin.close()
        terminal.stdout.close()
        terminal.stderr.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--alloc-cli", type=Path)
    parser.add_argument("--router", default=shutil.which("zenohd"))
    parser.add_argument("--cycles", default=300, type=int)
    args = parser.parse_args()
    if not args.router:
        parser.error("zenohd is required; specify --router /path/to/zenohd")
    check_serial_terminal()
    port = free_port()
    endpoint = f"tcp/127.0.0.1:{port}"
    clients = []
    with tempfile.TemporaryDirectory(prefix="zenoh-app-cli-smoke-") as tmp:
        with open(Path(tmp) / "router.log", "wb") as log:
            router = subprocess.Popen(
                [
                    args.router,
                    "--listen",
                    endpoint,
                    "--no-multicast-scouting",
                    "--cfg",
                    "scouting/multicast/enabled:false",
                    "--cfg",
                    "scouting/gossip/enabled:false",
                ],
                stdout=log,
                stderr=log,
            )
            try:
                for _ in range(100):
                    if router.poll() is not None:
                        raise AssertionError((Path(tmp) / "router.log").read_text())
                    try:
                        with socket.create_connection(("127.0.0.1", port), 0.1):
                            break
                    except OSError:
                        time.sleep(0.05)
                else:
                    raise AssertionError("isolated router did not start")
                for binary in [args.cli] + ([args.alloc_cli] if args.alloc_cli else []):
                    print(f"Testing {binary}", flush=True)
                    a, b = Console(binary, endpoint), Console(binary, endpoint)
                    clients.extend([a, b])
                    assert b"--timeout-ms" in a.command("help z_get")
                    assert b"z_broker" in a.command("--help")
                    assert b"session open" in a.command("z_ope\t")
                    assert b"session already open" in a.command("\x1b[A")
                    mark = a.send("z_sub cli_smoke/cancel_queued\n\x03")
                    a.expect(b"cancelled", mark)
                    a.expect(b"zenoh> ", mark)
                    assert b"session=closed" in a.command("status")
                    b.command(
                        "z_queryable cli_smoke/** 'query response'", b"queryable ready"
                    )
                    for i in range(args.cycles):
                        out = a.command(
                            f"z_get cli_smoke/request/{i} --timeout-ms 1000"
                        )
                        assert b"query response" in out, out
                        if i == 0:
                            assert b"session open" in out, out
                    b.cancel()
                    for i in range(args.cycles):
                        opened = a.command(f"z_sub cli_smoke/sample/{i}", b"subscriber ready")
                        if i > 0:
                            assert b"session open" in opened, opened
                        mark = len(a.output)
                        # Subscriber declarations have no router acknowledgement; publish a short
                        # burst so this tests command reuse without assuming propagation order.
                        b.command(
                            f"z_pub cli_smoke/sample/{i} 'sample {i}' --count 3 --interval-ms 5"
                        )
                        out = a.expect(f'"sample {i}"'.encode(), mark)
                        assert f"cli_smoke/sample/{i}".encode() in out
                        a.cancel()
                    a.command("z_open")
                    for _ in range(20):
                        a.command("z_close")
                        a.command("z_open")
                    # Ctrl-C during a listen must permit the next connection.
                    a.command("z_close")
                    a.command(
                        f"z_open tcp/127.0.0.1:{free_port()} --listen", b"opening "
                    )
                    a.cancel()
                    a.command("z_open")
                    a.command("z_get cli_smoke/no_queryable --timeout-ms 30")
                    a.command("z_pub cli_smoke/publish hello --count 3 --interval-ms 1")
                    a.command("z_pub_thr cli_smoke/throughput --count 100 --size 32")
                    b.command("z_pong", b"subscriber ready")
                    a.command("z_ping --count 3 --size 16")
                    b.cancel()
                    a.command(
                        "z_querier cli_smoke/no_queryable --count 3 --timeout-ms 10 --interval-ms 1"
                    )
                    assert b"error:" in a.command("z_get")
                    assert b"line exceeds" in a.command(
                        "z_put cli_smoke/unsafe " + "x" * 513
                    )
                    a.command("z_put cli_smoke/recovered ok")
                    check_one_shot(binary, endpoint)
                    if args.alloc_cli and binary == args.alloc_cli:
                        for _ in range(3):
                            b.command("z_close")
                            one, two = free_port(), free_port()
                            b.command(
                                f"z_broker {endpoint} --listen-1 tcp/127.0.0.1:{one} --listen-2 tcp/127.0.0.1:{two}",
                                b"broker running",
                            )
                            c = Console(binary, f"tcp/127.0.0.1:{one}")
                            d = Console(binary, f"tcp/127.0.0.1:{two}")
                            clients.extend([c, d])
                            c.command("z_open")
                            d.command("z_open")
                            d.command("z_sub cli_smoke/broker", b"subscriber ready")
                            mark = len(d.output)
                            c.command(
                                "z_pub cli_smoke/broker forwarded --count 3 --interval-ms 20"
                            )
                            d.expect(b'"forwarded"', mark)
                            mark = len(d.output)
                            a.command(
                                "z_pub cli_smoke/broker upstream --count 3 --interval-ms 20"
                            )
                            d.expect(b'"upstream"', mark)
                            b.cancel()
                            c.close()
                            d.close()
                            b.command("z_put cli_smoke/after_broker ok")
                        print(
                            "Passed broker forwarding and repeated exclusive-mode cleanup",
                            flush=True,
                        )
                    a.close()
                    b.close()
                    print(
                        f"Passed {args.cycles} runtime-key gets, {args.cycles} subscribe/disconnect/reopen cycles, connection cancellation, and scenario checks",
                        flush=True,
                    )
            finally:
                for client in clients:
                    client.close()
                router.terminate()
                try:
                    router.wait(5)
                except subprocess.TimeoutExpired:
                    router.kill()
                    router.wait()


if __name__ == "__main__":
    main()
