#!/usr/bin/env python3
"""Exercise unattended binaries through environment settings, with stdin closed."""
import argparse
import os
from pathlib import Path
import select
import socket
import subprocess
import tempfile
import time
from cli_smoke import Console, free_port

SETTING_NAMES = "SCENARIO KEY_EXPR PEER_KEY VALUE PARAMETERS CLOSE_DELAY_MS INTERVAL_MS TIMEOUT_MS COUNT PAYLOAD LISTEN BROKER_LISTEN_1 BROKER_LISTEN_2".split()

class Application:
    def __init__(self, binary, endpoint, scenario, **settings):
        self.output = b""
        env = {key: value for key, value in os.environ.items() if key not in SETTING_NAMES}
        env.update(ENDPOINT=endpoint, SCENARIO=scenario, RUST_LOG="error")
        env.update(settings)
        self.process = subprocess.Popen([str(binary)], stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                        stderr=subprocess.STDOUT, env=env)
    def expect(self, token, timeout=10):
        deadline = time.monotonic() + timeout
        while token not in self.output:
            left = deadline - time.monotonic()
            if left <= 0 or not select.select([self.process.stdout], [], [], left)[0]:
                raise AssertionError((token, self.output[-2000:]))
            data = os.read(self.process.stdout.fileno(), 8192)
            if not data: raise AssertionError((self.process.poll(), self.output))
            self.output += data
        return self.output
    def complete(self, expected=0):
        tail, _ = self.process.communicate(timeout=15)
        self.output += tail
        assert self.process.returncode == expected, self.output
        assert b"zenoh>" not in self.output and b"type help" not in self.output, self.output
        return self.output
    def close(self):
        if self.process.poll() is None:
            self.process.terminate()
            try: self.process.wait(3)
            except subprocess.TimeoutExpired: self.process.kill(); self.process.wait()
        self.process.stdout.close()

def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ["host", "fixed", "alloc"]: p.add_argument("--" + name, required=True, type=Path)
    p.add_argument("--router", required=True)
    args = p.parse_args()
    endpoint = f"tcp/127.0.0.1:{free_port()}"
    applications, hosts = [], []
    with tempfile.TemporaryDirectory(prefix="zenoh-headless-") as tmp:
        with open(Path(tmp) / "router.log", "wb") as log:
            router = subprocess.Popen([args.router, "--listen", endpoint, "--no-multicast-scouting",
                "--cfg", "scouting/multicast/enabled:false", "--cfg", "scouting/gossip/enabled:false"], stdout=log, stderr=log)
            try:
                port = int(endpoint.rsplit(":", 1)[1])
                for _ in range(100):
                    try:
                        with socket.create_connection(("127.0.0.1", port), .1): break
                    except OSError: time.sleep(.05)
                else: raise AssertionError((Path(tmp) / "router.log").read_text())
                host = Console(args.host, endpoint); hosts.append(host)
                for binary in [args.fixed, args.alloc]:
                    def start(scenario, **settings):
                        app = Application(binary, endpoint, scenario, **settings); applications.append(app); return app
                    host.command("z_sub headless/pub", b"subscriber ready")
                    mark = len(host.output)
                    app = start("pub", KEY_EXPR="headless/pub", VALUE="headless payload", COUNT="3", INTERVAL_MS="10")
                    assert b"published=3" in app.complete()
                    deadline = time.monotonic() + 5
                    while host.output[mark:].count(b'"headless payload"') < 3:
                        assert time.monotonic() < deadline, host.output[-2000:]
                        if host.selector.select(.1): host.output += os.read(host.process.stdout.fileno(), 8192)
                    host.cancel()
                    host.command("z_sub headless/put", b"subscriber ready")
                    mark = len(host.output)
                    start("put", KEY_EXPR="headless/put", VALUE="single sample").complete()
                    host.expect(b'"single sample"', mark); host.cancel()
                    app = start("sub", KEY_EXPR="headless/sub", COUNT="3")
                    app.expect(b"subscriber ready")
                    host.command("z_pub headless/sub source --count 10 --interval-ms 20")
                    assert b'"source"' in app.complete()
                    host.command("z_queryable headless/** reply", b"queryable ready")
                    for scenario in ["get", "querier"]:
                        app = start(scenario, KEY_EXPR="headless/get", COUNT="3", INTERVAL_MS="1")
                        assert b'"reply"' in app.complete()
                    host.cancel()
                    app = start("queryable", KEY_EXPR="headless/query", VALUE="board-style reply", COUNT="1")
                    app.expect(b"queryable ready"); time.sleep(.1)
                    assert b'"board-style reply"' in host.command("z_get headless/query")
                    app.complete()
                    host.command("z_pong", b"subscriber ready")
                    app = start("ping", COUNT="3", PAYLOAD="16")
                    assert b"rtt_us=" in app.complete(); host.cancel()
                    app = start("pong", COUNT="3")
                    app.expect(b"subscriber ready")
                    assert b"rtt_us=" in host.command("z_ping --count 3 --size 16")
                    app.complete()
                    app = start("pub_thr", COUNT="100", PAYLOAD="32")
                    assert b"published=100" in app.complete()
                    app = start("sub_thr", KEY_EXPR="headless/thr", COUNT="3")
                    app.expect(b"subscriber ready")
                    host.command("z_pub headless/thr throughput --count 10 --interval-ms 20")
                    assert b"received=" in app.complete()
                    app = start("open"); app.expect(b"session open"); app.close()
                    start("unknown").complete(2)
                    start("pub", COUNT="0").complete(2)
                    if binary == args.fixed: start("broker").complete(2)
                    print(f"Headless {binary.name}: publish/subscribe, get/querier/queryable, ping/pong, throughput, open and validation passed", flush=True)
                one, two = free_port(), free_port()
                app = Application(args.alloc, endpoint, "broker", BROKER_LISTEN_1=f"tcp/127.0.0.1:{one}", BROKER_LISTEN_2=f"tcp/127.0.0.1:{two}")
                applications.append(app); app.expect(b"broker running")
                c, d = Console(args.host, f"tcp/127.0.0.1:{one}"), Console(args.host, f"tcp/127.0.0.1:{two}")
                hosts.extend([c, d]); c.command("z_open"); d.command("z_open")
                d.command("z_sub headless/broker", b"subscriber ready")
                mark = len(d.output); c.command("z_pub headless/broker forwarded --count 5 --interval-ms 20")
                d.expect(b'"forwarded"', mark)
                mark = len(d.output); host.command("z_pub headless/broker upstream --count 5 --interval-ms 20")
                d.expect(b'"upstream"', mark)
                print("Headless alloc broker: client and upstream forwarding passed", flush=True)
            finally:
                for app in applications: app.close()
                for host in hosts: host.close()
                router.terminate(); router.wait(5)

if __name__ == "__main__": main()
