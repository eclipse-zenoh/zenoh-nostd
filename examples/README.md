# Startup scenarios

From the repository root inside `nix develop`, start a local router, then run:

```sh
SCENARIO=pub COUNT=3 just std-headless
COUNT=3 just build-esp32c6-headless pub
COUNT=3 just build-esp32s3-headless pub
COUNT=3 just build-stm32-headless pub
COUNT=3 just wasm-headless pub
```

MCU settings, including a reachable ENDPOINT and ESP Wi-Fi credentials, are
compiled into the firmware. Native settings are read at startup. Build-only
recipes do not flash. The matching recipes without `build-` flash MCU firmware.
Add `alloc` after the scenario for allocated storage. Scenarios are shared typed
operations in `common/src/scenarios`; each target now builds one `zenoh-app`.
