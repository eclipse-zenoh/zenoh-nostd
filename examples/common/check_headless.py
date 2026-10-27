#!/usr/bin/env python3
"""Check the headless feature matrix and reject frontend dependencies."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
env = {**os.environ, "CARGO_TARGET_DIR": str(root / "target"),
       "WIFI_PASSWORD": "ci-placeholder", "ENDPOINT": "tcp/192.0.2.1:7447"}
for project in ["common", "embassy", "native", "wasm", "embassy/esp/esp32s3-wifi", "embassy/esp/esp32c6-pico-wifi", "embassy/stm32/nucleo-h753zi-ethernet"]:
    cargo = ["cargo"] + (["+esp"] if project == "embassy/esp/esp32s3-wifi" else [])
    for features in ["", "alloc"]:
        flags = ["--locked", "--no-default-features", "--features", features]
        path = root / "examples" / project
        subprocess.run(cargo + ["check", *flags], cwd=path, env=env, check=True)
        tree = subprocess.check_output(cargo + ["tree", *flags, "--edges", "normal", "--prefix", "none"],
                                       cwd=path, env=env, text=True)
        assert not any(line.startswith(("embedded-cli ", "embedded-cli-macros ", "embassy-usb ")) for line in tree.splitlines()), project
        print(f"Headless {project}, {features or 'fixed'}: frontend absent", flush=True)
