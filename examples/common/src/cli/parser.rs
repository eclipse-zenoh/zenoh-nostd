use heapless::{String, Vec};

pub const LINE_CAPACITY: usize = 512;
pub use crate::scenarios::{KEY_CAPACITY, PAYLOAD_CAPACITY};
pub type Line = String<LINE_CAPACITY>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Status,
    Open,
    Close,
    Quit,
    Put,
    Get,
    Pub,
    Sub,
    Queryable,
    Querier,
    Ping,
    Pong,
    PubThr,
    SubThr,
    Broker,
}

#[derive(Debug)]
pub struct Command {
    pub kind: Kind,
    pub key: String<KEY_CAPACITY>,
    pub peer_key: String<KEY_CAPACITY>,
    pub payload: String<PAYLOAD_CAPACITY>,
    pub endpoint: String<128>,
    pub parameters: String<128>,
    pub listen1: String<128>,
    pub listen2: String<128>,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    pub count: Option<u64>,
    pub size: usize,
    pub listen: bool,
}

/// Command names, arguments, help and completion share one definition.
#[derive(Debug, embedded_cli::Command)]
pub(super) enum Schema<'a> {
    /// Show connection state, capacities and dropped output.
    Status,
    /// Open a session using the default or supplied endpoint.
    #[command(name = "z_open")]
    Open {
        endpoint: Option<&'a str>,
        /// Accept an incoming connection instead of connecting.
        #[arg(long)]
        listen: bool,
    },
    /// Close the session and release its resources.
    #[command(name = "z_close")]
    Close,
    /// Exit the application (reset an MCU to restart).
    Quit,
    /// Alias for quit.
    Exit,
    /// Publish one value.
    #[command(name = "z_put")]
    Put { key: &'a str, payload: &'a str },
    /// Query values until finalization or timeout.
    #[command(name = "z_get")]
    Get {
        key: &'a str,
        #[arg(long, default_value = "")]
        parameters: &'a str,
        #[arg(long, default_value_t = 2000)]
        timeout_ms: u64,
    },
    /// Publish repeatedly; Ctrl-C stops an unbounded run.
    #[command(name = "z_pub")]
    Pub {
        key: &'a str,
        payload: &'a str,
        #[arg(long, default_value_t = 1000)]
        interval_ms: u64,
        #[arg(long)]
        count: Option<u64>,
    },
    /// Receive matching samples until Ctrl-C.
    #[command(name = "z_sub")]
    Sub { key: &'a str },
    /// Answer queries with the supplied payload until Ctrl-C.
    #[command(name = "z_queryable")]
    Queryable { key: &'a str, payload: &'a str },
    /// Repeat queries through a reusable querier.
    #[command(name = "z_querier")]
    Querier {
        key: &'a str,
        #[arg(long, default_value = "")]
        parameters: &'a str,
        #[arg(long, default_value_t = 2000)]
        timeout_ms: u64,
        #[arg(long, default_value_t = 1000)]
        interval_ms: u64,
        #[arg(long)]
        count: Option<u64>,
    },
    /// Publish pings and measure reply latency.
    #[command(name = "z_ping")]
    Ping {
        #[arg(default_value = "test/ping")]
        key: &'a str,
        #[arg(long, default_value = "test/pong")]
        peer_key: &'a str,
        #[arg(long, default_value_t = 8)]
        size: usize,
        #[arg(long, default_value_t = 100)]
        count: u64,
        #[arg(long, default_value_t = 2000)]
        timeout_ms: u64,
    },
    /// Echo samples to the peer key until Ctrl-C.
    #[command(name = "z_pong")]
    Pong {
        #[arg(default_value = "test/ping")]
        key: &'a str,
        #[arg(long, default_value = "test/pong")]
        peer_key: &'a str,
    },
    /// Publish throughput traffic; Ctrl-C stops an unbounded run.
    #[command(name = "z_pub_thr")]
    PubThr {
        #[arg(default_value = "test/thr")]
        key: &'a str,
        #[arg(long, default_value_t = 8)]
        size: usize,
        #[arg(long)]
        count: Option<u64>,
    },
    /// Count received throughput traffic until Ctrl-C or the count limit.
    #[command(name = "z_sub_thr")]
    SubThr {
        #[arg(default_value = "test/thr")]
        key: &'a str,
        #[arg(long)]
        count: Option<u64>,
    },
    /// Run an exclusive broker; requires alloc and listening support.
    #[command(name = "z_broker")]
    Broker {
        upstream: Option<&'a str>,
        #[arg(long = "listen-1", default_value = "")]
        listen1: &'a str,
        #[arg(long = "listen-2", default_value = "")]
        listen2: &'a str,
    },
}

impl Command {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            key: String::new(),
            peer_key: String::new(),
            payload: String::new(),
            endpoint: String::new(),
            parameters: String::new(),
            listen1: String::new(),
            listen2: String::new(),
            interval_ms: 1000,
            timeout_ms: 2000,
            count: None,
            size: 8,
            listen: false,
        }
    }
    pub(super) fn scenario(&self) -> Option<crate::scenarios::Options<'_>> {
        use crate::scenarios::{Options, Scenario};
        let scenario = match self.kind {
            Kind::Open => Scenario::Open,
            Kind::Put => Scenario::Put,
            Kind::Get => Scenario::Get,
            Kind::Pub => Scenario::Pub,
            Kind::Sub => Scenario::Sub,
            Kind::Queryable => Scenario::Queryable,
            Kind::Querier => Scenario::Querier,
            Kind::Ping => Scenario::Ping,
            Kind::Pong => Scenario::Pong,
            Kind::PubThr => Scenario::PubThr,
            Kind::SubThr => Scenario::SubThr,
            Kind::Broker => Scenario::Broker,
            _ => return None,
        };
        Some(Options {
            scenario,
            key: &self.key,
            peer_key: &self.peer_key,
            payload: self.payload.as_bytes(),
            parameters: &self.parameters,
            close_delay_ms: 0,
            interval_ms: self.interval_ms,
            timeout_ms: self.timeout_ms,
            count: self.count,
            size: self.size,
            listen: self.listen,
            listen1: if self.listen1.is_empty() {
                crate::BROKER_LISTEN_1
            } else {
                &self.listen1
            },
            listen2: if self.listen2.is_empty() {
                crate::BROKER_LISTEN_2
            } else {
                &self.listen2
            },
        })
    }
    pub(super) fn quit() -> Self {
        Self::new(Kind::Quit)
    }
}

impl TryFrom<Schema<'_>> for Command {
    type Error = &'static str;
    fn try_from(schema: Schema<'_>) -> Result<Self, Self::Error> {
        let (kind, key, payload) = match &schema {
            Schema::Status => (Kind::Status, "", ""),
            Schema::Open { .. } => (Kind::Open, "", ""),
            Schema::Close => (Kind::Close, "", ""),
            Schema::Quit | Schema::Exit => (Kind::Quit, "", ""),
            Schema::Put { key, payload } => (Kind::Put, *key, *payload),
            Schema::Get { key, .. } => (Kind::Get, *key, ""),
            Schema::Pub { key, payload, .. } => (Kind::Pub, *key, *payload),
            Schema::Sub { key } => (Kind::Sub, *key, ""),
            Schema::Queryable { key, payload } => (Kind::Queryable, *key, *payload),
            Schema::Querier { key, .. } => (Kind::Querier, *key, ""),
            Schema::Ping { key, .. } => (Kind::Ping, *key, ""),
            Schema::Pong { key, .. } => (Kind::Pong, *key, ""),
            Schema::PubThr { key, .. } => (Kind::PubThr, *key, ""),
            Schema::SubThr { key, .. } => (Kind::SubThr, *key, ""),
            Schema::Broker { .. } => (Kind::Broker, "", ""),
        };
        let mut cmd = Self::new(kind);
        cmd.key.push_str(key).map_err(|_| "key exceeds 128 bytes")?;
        cmd.payload
            .push_str(payload)
            .map_err(|_| "payload exceeds 256 bytes")?;
        match schema {
            Schema::Open { endpoint, listen } => {
                cmd.endpoint
                    .push_str(endpoint.unwrap_or(""))
                    .map_err(|_| "endpoint exceeds 128 bytes")?;
                cmd.listen = listen;
            }
            Schema::Get {
                parameters,
                timeout_ms,
                ..
            } => {
                cmd.parameters
                    .push_str(parameters)
                    .map_err(|_| "parameters exceed 128 bytes")?;
                cmd.timeout_ms = timeout_ms;
            }
            Schema::Pub {
                interval_ms, count, ..
            } => {
                cmd.interval_ms = interval_ms;
                cmd.count = count;
            }
            Schema::Querier {
                parameters,
                timeout_ms,
                interval_ms,
                count,
                ..
            } => {
                cmd.parameters
                    .push_str(parameters)
                    .map_err(|_| "parameters exceed 128 bytes")?;
                cmd.timeout_ms = timeout_ms;
                cmd.interval_ms = interval_ms;
                cmd.count = count;
            }
            Schema::Ping {
                peer_key,
                size,
                count,
                timeout_ms,
                ..
            } => {
                cmd.peer_key
                    .push_str(peer_key)
                    .map_err(|_| "peer key exceeds 128 bytes")?;
                cmd.size = size;
                cmd.count = Some(count);
                cmd.timeout_ms = timeout_ms;
            }
            Schema::Pong { peer_key, .. } => {
                cmd.peer_key
                    .push_str(peer_key)
                    .map_err(|_| "peer key exceeds 128 bytes")?;
            }
            Schema::PubThr { size, count, .. } => {
                cmd.size = size;
                cmd.count = count;
            }
            Schema::SubThr { count, .. } => {
                cmd.count = count;
            }
            Schema::Broker {
                upstream,
                listen1,
                listen2,
            } => {
                cmd.endpoint
                    .push_str(upstream.unwrap_or(""))
                    .map_err(|_| "endpoint exceeds 128 bytes")?;
                cmd.listen1
                    .push_str(listen1)
                    .map_err(|_| "endpoint exceeds 128 bytes")?;
                cmd.listen2
                    .push_str(listen2)
                    .map_err(|_| "endpoint exceeds 128 bytes")?;
            }
            _ => {}
        }
        for number in [Some(cmd.interval_ms), Some(cmd.timeout_ms), cmd.count]
            .into_iter()
            .flatten()
        {
            if number == 0 || number > 86_400_000 {
                return Err("numeric argument out of range (1..86400000)");
            }
        }
        if cmd.size == 0 || cmd.size > PAYLOAD_CAPACITY {
            return Err("payload size out of range (1..256)");
        }
        if !matches!(
            kind,
            Kind::Status | Kind::Open | Kind::Close | Kind::Quit | Kind::Broker
        ) {
            zenoh_nostd::session::zenoh::keyexpr::new(&cmd.key)
                .map_err(|_| "invalid key expression")?;
            if matches!(kind, Kind::Put | Kind::Pub | Kind::PubThr | Kind::Ping)
                && cmd.key.contains('*')
            {
                return Err("publishing key must not contain wildcards");
            }
        }
        if matches!(kind, Kind::Ping | Kind::Pong) {
            zenoh_nostd::session::zenoh::keyexpr::new(&cmd.peer_key)
                .map_err(|_| "invalid peer key")?;
            if cmd.peer_key.contains('*') {
                return Err("peer publishing key must not contain wildcards");
            }
        }
        Ok(cmd)
    }
}

// embedded-cli 0.2.1 accepts repeated options and a dangling option with a
// default, and ignores arguments on unit variants. Keep our stricter contract.
pub(super) fn validate_raw(
    raw: &embedded_cli::command::RawCommand<'_>,
) -> Result<(), &'static str> {
    use embedded_cli::arguments::Arg;
    let mut seen = Vec::<&str, 10>::new();
    let mut pending = false;
    let mut count = 0;
    for arg in raw.args().args() {
        count += 1;
        if count > 19 {
            return Err("too many arguments");
        }
        if matches!(raw.name(), "status" | "z_close" | "quit" | "exit") {
            return Err("this command takes no arguments");
        }
        match arg {
            Ok(Arg::LongOption(name)) => {
                if pending {
                    return Err("missing option value");
                }
                if seen.contains(&name) {
                    return Err("duplicate option");
                }
                seen.push(name).map_err(|_| "too many options")?;
                pending = name != "listen";
            }
            Ok(Arg::Value(_)) => pending = false,
            Ok(Arg::DoubleDash) | Ok(Arg::ShortOption(_)) if pending => {
                return Err("missing option value");
            }
            _ => {}
        }
    }
    if pending {
        return Err("missing option value");
    }
    Ok(())
}

// Preserve our shell-style quoting while embedded-cli handles the command
// schema, typed arguments, help, completion and interactive editor. Its native
// tokenizer only understands double quotes and accepts incomplete quotes.
// This preflight rejects malformed input and emits its supported spelling.
pub(super) const EDIT_CAPACITY: usize = LINE_CAPACITY * 2 + 64;
pub(super) fn normalize(input: &str) -> Result<String<EDIT_CAPACITY>, &'static str> {
    let (storage, ranges) = tokens(input)?;
    let mut output = String::new();
    for (i, range) in ranges.into_iter().enumerate() {
        if i != 0 {
            output.push(' ').map_err(|_| "normalized input too long")?;
        }
        let token = &storage[range];
        let token = if i == 0 && matches!(token, "--help" | "-h") {
            "help"
        } else {
            token
        };
        if token.is_empty()
            || token
                .bytes()
                .any(|b| matches!(b, b' ' | b'\t' | b'"' | b'\\'))
        {
            output.push('"').map_err(|_| "normalized input too long")?;
            for c in token.chars() {
                if matches!(c, '"' | '\\') {
                    output.push('\\').map_err(|_| "normalized input too long")?;
                }
                output.push(c).map_err(|_| "normalized input too long")?;
            }
            output.push('"').map_err(|_| "normalized input too long")?;
        } else {
            output
                .push_str(token)
                .map_err(|_| "normalized input too long")?;
        }
    }
    Ok(output)
}

// Store token ranges in one buffer instead of allocating one full line per token.
fn tokens(input: &str) -> Result<(Line, Vec<core::ops::Range<usize>, 20>), &'static str> {
    if input.len() > LINE_CAPACITY {
        return Err("line exceeds 512 bytes");
    }
    let mut storage = Line::new();
    let mut ranges = Vec::new();
    let mut start = None;
    let mut quote = None;
    let mut escaped = false;
    for ch in input.chars() {
        if escaped {
            storage.push(ch).map_err(|_| "line too long")?;
            escaped = false;
        } else if ch == '\\' && quote != Some('\'') {
            start.get_or_insert(storage.len());
            escaped = true;
        } else if let Some(q) = quote {
            if ch == q {
                quote = None;
            } else {
                storage.push(ch).map_err(|_| "line too long")?;
            }
        } else if ch == '\'' || ch == '"' {
            start.get_or_insert(storage.len());
            quote = Some(ch);
        } else if ch.is_ascii_whitespace() {
            if let Some(begin) = start.take() {
                ranges
                    .push(begin..storage.len())
                    .map_err(|_| "too many arguments")?;
            }
        } else {
            start.get_or_insert(storage.len());
            storage.push(ch).map_err(|_| "line too long")?;
        }
    }
    if quote.is_some() || escaped {
        return Err("unfinished quote or escape");
    }
    if let Some(begin) = start {
        ranges
            .push(begin..storage.len())
            .map_err(|_| "too many arguments")?;
    }
    Ok((storage, ranges))
}
