# Change: Automatic Reconnection with Exponential Backoff

## Why

zenoh-pico supports `Z_FEATURE_AUTO_RECONNECT` since v1.0.0. zenoh-nostd has no reconnection logic — if a transport disconnects (TCP reset, WiFi dropout, UART cable pull, Ethernet link down), the session transitions to `Closed` or `Failed` with no recovery path. The application must manually create a new session.

For embedded and IoT deployments, network connectivity is unreliable. Devices in the field may experience:
- Temporary WiFi disconnects (router reboot, interference)
- TCP keepalive expirations on long-idle links
- Physical cable disconnections (serial, Ethernet)
- Power cycling of network infrastructure
- Mobile nodes moving between access points

Without auto-reconnect, every transient network interruption becomes a permanent failure requiring manual intervention or a full device reset — unacceptable for unattended deployments.

## What Changes

### 1. Session state machine — Degraded state and reconnect

Extend `SessionState`:

```rust
pub enum SessionState {
    Closed,
    Opening,
    Open,
    Degraded { retry_count: u32, next_retry_at: Instant },
    Closing,
    Failed(SessionError),
}
```

`Degraded` is the state entered when a transport disconnects mid-session (not during initial opening). From `Degraded`, the state machine can:
- Transition to `Open` (successful reconnect) — restores the session to full function
- Transition to `Failed(SessionError::ReconnectExhausted)` (max retries reached)
- Transition to `Closing` (user-requested close during degraded state)

Add state transition rules in `zenoh-sansio`:

```rust
impl SessionStateMachine {
    pub fn on_transport_loss(&mut self) -> Result<(), SessionError> {
        if self.state != SessionState::Open {
            return Err(SessionError::NotConnected);
        }
        // Only transition to Degraded if auto-reconnect is enabled
        if self.config.auto_reconnect.enabled() {
            self.state = SessionState::Degraded {
                retry_count: 0,
                next_retry_at: Instant::now() + self.config.auto_reconnect.initial_delay(),
            };
            Ok(())
        } else {
            self.state = SessionState::Failed(SessionError::TransportLost);
            Err(SessionError::TransportLost)
        }
    }

    pub fn on_reconnect_success(&mut self) -> Result<(), SessionError> {
        match self.state {
            SessionState::Degraded { .. } => {
                self.state = SessionState::Open;
                Ok(())
            }
            _ => Err(SessionError::InvalidState),
        }
    }

    pub fn on_reconnect_failure(&mut self) -> Result<(), SessionError> {
        match self.state {
            SessionState::Degraded { retry_count, .. } => {
                if retry_count >= self.config.auto_reconnect.max_retries() {
                    self.state = SessionState::Failed(SessionError::ReconnectExhausted);
                    return Err(SessionError::ReconnectExhausted);
                }
                let delay = self.config.auto_reconnect.next_delay(retry_count);
                self.state = SessionState::Degraded {
                    retry_count: retry_count + 1,
                    next_retry_at: Instant::now() + delay,
                };
                Ok(())
            }
            _ => Err(SessionError::InvalidState),
        }
    }
}
```

### 2. Reconnection policy configuration

Add reconnection policy types to `zenoh-nostd`:

```rust
#[derive(Clone, Debug)]
pub struct ReconnectPolicy {
    /// Maximum number of retry attempts. 0 = disabled, None = infinite.
    pub max_retries: Option<u32>,
    /// Initial delay before first retry (ms). Default: 1000.
    pub initial_delay_ms: u64,
    /// Maximum delay between retries (ms). Default: 30000.
    pub max_delay_ms: u64,
    /// Backoff multiplier. Default: 2.0 (exponential).
    pub backoff_factor: f64,
    /// Jitter fraction (0.0–1.0) to randomize delay. Default: 0.1 (10%).
    pub jitter_fraction: f64,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            max_retries: Some(5),     // 5 retries by default
            initial_delay_ms: 1000,   // 1 second
            max_delay_ms: 30000,      // 30 seconds
            backoff_factor: 2.0,      // 1s, 2s, 4s, 8s, 16s, 30s...
            jitter_fraction: 0.1,     // ±10% jitter
        }
    }
}

impl ReconnectPolicy {
    pub fn next_delay(&self, retry_count: u32) -> Duration {
        let delay = self.initial_delay_ms as f64
            * self.backoff_factor.powi(retry_count as i32);
        let delay = delay.min(self.max_delay_ms as f64);
        let jitter = delay * self.jitter_fraction * (random_fraction() * 2.0 - 1.0);
        Duration::from_millis((delay + jitter) as u64)
    }

    pub fn enabled(&self) -> bool {
        self.max_retries != Some(0)
    }
}
```

Key design choices:
- `f64` for backoff calculations is acceptable in `std` builds; for `no_std`, use fixed-point arithmetic or a precomputed backoff table — gate with `#[cfg(feature = "std")]` and provide integer-only alternative
- Jitter uses `random_fraction()` which requires an RNG source. For `no_std` without alloc, use an LFSR or simple hash-based jitter; gate behind `rand` feature
- All duration calculations use saturating arithmetic — no overflow paths

### 3. zenoh-sansio — Feature flag `auto-reconnect`

Add to `crates/zenoh-sansio/Cargo.toml`:

```toml
[features]
auto-reconnect = []
```

Gate the `Degraded` state transitions and `ReconnectPolicy` behind this flag. The `SessionState` enum variant `Degraded` is always present (minimal memory cost: a discriminant + payload) but transitions into and out of it are gated. When `auto-reconnect` is disabled and transport loss occurs, the session goes directly to `Failed`.

### 4. zenoh-nostd — Reconnect task/worker

Add a reconnect worker that runs in the session event loop:

```rust
pub struct ReconnectWorker<LinkManager, Buff> {
    state: ReconnectState,
    config: ReconnectPolicy,
    endpoint: Endpoint<'static>,
    link_manager: Arc<LinkManager>,
    transport_builder: TransportBuilder,
    /// Session state shared with the main event loop
    session_handle: SessionHandle,
    /// Callbacks for connection state changes
    callbacks: ConnectionCallbacks,
}

enum ReconnectState {
    Idle,
    Backoff { retry_count: u32, deadline: Instant },
    Connecting,
}
```

The worker is driven by the session's event loop on each tick:

```rust
impl<LM, Buff> ReconnectWorker<LM, Buff>
where
    LM: ZLinkManager,
    Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
{
    pub fn tick(&mut self, now: Instant) -> Result<ReconnectAction, ReconnectError> {
        match self.state {
            ReconnectState::Idle => Ok(ReconnectAction::None),
            ReconnectState::Backoff { retry_count, deadline } => {
                if now >= deadline {
                    self.state = ReconnectState::Connecting;
                    Ok(ReconnectAction::Connect {
                        endpoint: self.endpoint.clone(),
                    })
                } else {
                    Ok(ReconnectAction::Wait(deadline - now))
                }
            }
            ReconnectState::Connecting => {
                // Connect is driven externally — wait for result
                Ok(ReconnectAction::None)
            }
        }
    }
}
```

### 5. Wire reconnect into transport loss handling

When the transport layer detects link loss (via keepalive timeout, read error, or explicit close):

1. Session event loop calls `on_transport_loss()` on the state machine
2. If `auto-reconnect` is enabled and state transitions to `Degraded`, spawn reconnect worker
3. Reconnect worker computes backoff delay and sets timer
4. On timer expiry, attempt to re-establish the transport link:
   a. Reconnect to the original endpoint
   b. Run Zenoh handshake (`InitSyn`/`InitAck`/`OpenSyn`/`OpenAck`)
   c. Re-declare session state (subscriptions, queryables, publishers)
5. On success: transition to `Open`, notify callbacks
6. On failure: call `on_reconnect_failure()`, compute next backoff, retry

### 6. Connection state callbacks

Add configurable callbacks for connection state transitions:

```rust
pub struct ConnectionCallbacks {
    /// Called when connection is established (initial or reconnection)
    pub on_connected: Option<fn(&SessionHandle)>,
    /// Called when connection is lost
    pub on_disconnected: Option<fn(&SessionHandle, DisconnectReason)>,
    /// Called on each reconnect attempt
    pub on_reconnecting: Option<fn(&SessionHandle, u32, Duration)>,
    /// Called when reconnection fails (max retries exhausted)
    pub on_reconnect_failed: Option<fn(&SessionHandle, u32, SessionError)>,
}
```

These callbacks are stored in the session config and invoked from the event loop. For `no_std` without alloc, callbacks are function pointers (no closures) to avoid allocation.

### 7. State re-declaration on reconnect

After successful reconnection, the session must re-declare all active state:

- **Subscriptions**: Re-send `DeclareSubscriber` for all active subscriber key expressions
- **Queryables**: Re-send `DeclareQueryable` for all active queryable key expressions
- **Publishers**: Re-send `DeclarePublisher` for all active publisher key expressions
- **Liveliness tokens**: Re-declare liveliness tokens if liveliness is implemented

Add `SessionState::declarations()` that returns a list of active declarations to replay:

```rust
#[cfg(feature = "auto-reconnect")]
impl SessionState {
    pub fn active_declarations(&self) -> &[Declaration] {
        &self.declarations
    }
}
```

The replay is performed by the reconnect worker after the transport handshake and before signaling `on_connected`. This matches zenoh-pico's `zp_re_declare_session` behavior.

### 8. Session config integration

Add to `ZSessionConfig`:

```rust
impl ZSessionConfig {
    pub fn with_auto_reconnect(mut self, policy: ReconnectPolicy) -> Self {
        self.auto_reconnect = Some(policy);
        self
    }

    pub fn with_reconnect_callbacks(mut self, callbacks: ConnectionCallbacks) -> Self {
        self.reconnect_callbacks = Some(callbacks);
        self
    }
}
```

### 9. Tests

| Test | Scope | Method |
|---|---|---|
| ReconnectPolicy delay calculation | `zenoh-nostd` | Unit test: verify exponential backoff with jitter bounds, max delay cap, zero retries disables |
| Session state machine: transport loss → Degraded | `zenoh-sansio` | Unit test: verify state transitions with and without auto-reconnect |
| Session state machine: Degraded → Open (success) | `zenoh-sansio` | Unit test: verify `on_reconnect_success` transitions to Open |
| Session state machine: Degraded → Failed (exhausted) | `zenoh-sansio` | Unit test: verify max retries transitions to Failed |
| Session state machine: invalid transitions | `zenoh-sansio` | Unit test: verify error on unexpected `on_reconnect_success`/`on_reconnect_failure` |
| ReconnectWorker tick timing | `zenoh-nostd` | Unit test: advance time, verify `ReconnectAction::Connect` fires at correct deadline |
| ReconnectWorker idle state | `zenoh-nostd` | Unit test: no action when idle |
| Full reconnect cycle | `zenoh-nostd` | Integration test: connect, kill transport, verify reconnect with backoff, verify state re-declaration |
| Reconnect with state replay | `zenoh-nostd` | Integration test: subscribe to keyexpr, disconnect, reconnect, verify subscription is re-declared and messages resume |
| Callback invocations | `zenoh-nostd` | Unit test: verify `on_connected`, `on_disconnected`, `on_reconnecting`, `on_reconnect_failed` fire correctly |

### 10. Property tests

```rust
proptest! {
    #[test]
    fn reconnect_policy_delays_are_bounded(
        max_retries in 0u32..100u32,
        initial_delay in 1u64..10000u64,
        max_delay in 100u64..120000u64,
        backoff in 1.0f64..10.0f64,
    ) {
        let policy = ReconnectPolicy {
            max_retries: Some(max_retries),
            initial_delay_ms: initial_delay,
            max_delay_ms: max_delay,
            backoff_factor: backoff,
            jitter_fraction: 0.1,
        };
        for i in 0..max_retries {
            let d = policy.next_delay(i);
            assert!(d.as_millis() <= max_delay as u128);
        }
    }

    #[test]
    fn reconnect_delays_monotonic(retries: Vec<u32>) {
        // Each subsequent retry delay should be >= previous (modulo jitter)
        // Lower bound: at least the same order of magnitude
    }
}
```

### 11. Kani harnesses

Add bounded verification for:
- `ReconnectPolicy::next_delay()`: no overflow, delay within bounds for all inputs up to max_retries=10
- Session state machine: all valid transitions from Degraded are total and no panic paths exist
- ReconnectWorker: finite state machine reaches terminal states within bounded steps
- Jitter calculation: no floating-point edge cases in std path; no overflow in fixed-point no_std path

## Capabilities

### New

- `auto-reconnect` — Automatic reconnection with configurable exponential backoff, jitter, max retries, and connection state callbacks
- `Degraded` session state — explicit state for mid-session transport loss with recovery path
- State re-declaration on reconnect — automatic replay of subscriptions, queryables, and publisher declarations

### Modified

- `zenoh-sansio`: `SessionState` gains `Degraded` variant; `SessionStateMachine` gains transport loss, reconnect success, reconnect failure transitions (gated behind `auto-reconnect` feature)
- `zenoh-nostd`: `ReconnectPolicy`, `ReconnectWorker`, `ConnectionCallbacks` types; session config integration; event loop integration

### Unchanged

- Initial session establishment path (first connect remains unchanged)
- Transport link types — reconnect creates new transport links with the same codec handshake
- Session API (`put`, `get`, `subscribe`, `declare_queryable`) — transparently resume after reconnect

## Impact

| Crate | Change | Risk |
|---|---|---|
| `zenoh-sansio` | `Degraded` state variant; gated transitions; feature flag | Low — additive, no existing transitions modified |
| `zenoh-nostd` | `ReconnectPolicy`, `ReconnectWorker`, `ConnectionCallbacks`; event loop integration; state replay | Medium — event loop integration requires careful async coordination |
| `openspec/protocol-features` | Add auto-reconnect to feature matrix | None |

## Known Gaps

- `no_std` jitter requires an RNG source — default implementation uses a simple LCG or LFSR (not cryptographically secure); users can supply their own via a trait
- State re-declaration on reconnect replays all declarations — no differential re-declaration (re-declare only what changed since disconnect). This is a future optimization
- Callbacks are function pointers only (no closures) to maintain `no_std` compatibility without alloc. For `std` builds, an `alloc`-based callback with closures could be added later behind a `std` feature gate
- If the underlying endpoint changes (e.g., DHCP reassigns IP), reconnect to the original endpoint will fail. Dynamic endpoint re-resolution is not implemented — users should monitor `on_reconnect_failed` and create a new session
- Reconnect during initial connection (Opening → Degraded) is not supported — transport loss during initial handshake is an immediate failure. The Degraded state only applies after an Open session drops