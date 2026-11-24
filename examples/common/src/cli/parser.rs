use heapless::{String, Vec};
use zenoh_cli::{LegacyZenohCommand, ZenohCommand};

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

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug, embedded_cli::CommandGroup)]
pub(super) enum Schema<'a> {
    Local(ConsoleCommand<'a>),
    Legacy(LegacyZenohCommand<'a>),
}

#[derive(Debug, embedded_cli::Command)]
pub(super) enum ConsoleCommand<'a> {
    /// Exit the application (reset an MCU to restart).
    Quit,
    /// Alias for quit.
    Exit,
    /// Run a Zenoh command.
    #[command(subcommand)]
    Zenoh(ZenohCommand<'a>),
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
        let command = match schema {
            Schema::Local(ConsoleCommand::Quit | ConsoleCommand::Exit) => return Ok(Self::quit()),
            Schema::Local(ConsoleCommand::Zenoh(command)) => command,
            Schema::Legacy(command) => command.into(),
        };
        let (kind, key, payload) = match &command {
            ZenohCommand::Status => (Kind::Status, "", ""),
            ZenohCommand::Open { .. } => (Kind::Open, "", ""),
            ZenohCommand::Close => (Kind::Close, "", ""),
            ZenohCommand::Put { key, payload } => (Kind::Put, *key, *payload),
            ZenohCommand::Get { key, .. } => (Kind::Get, *key, ""),
            ZenohCommand::Pub { key, payload, .. } => (Kind::Pub, *key, *payload),
            ZenohCommand::Sub { key } => (Kind::Sub, *key, ""),
            ZenohCommand::Queryable { key, payload } => (Kind::Queryable, *key, *payload),
            ZenohCommand::Querier { key, .. } => (Kind::Querier, *key, ""),
            ZenohCommand::Ping { key, .. } => (Kind::Ping, *key, ""),
            ZenohCommand::Pong { key, .. } => (Kind::Pong, *key, ""),
            ZenohCommand::PubThr { key, .. } => (Kind::PubThr, *key, ""),
            ZenohCommand::SubThr { key, .. } => (Kind::SubThr, *key, ""),
            ZenohCommand::Broker { .. } => (Kind::Broker, "", ""),
        };
        let mut cmd = Self::new(kind);
        cmd.key.push_str(key).map_err(|_| "key exceeds 128 bytes")?;
        cmd.payload
            .push_str(payload)
            .map_err(|_| "payload exceeds 256 bytes")?;
        match command {
            ZenohCommand::Open { endpoint, listen } => {
                cmd.endpoint
                    .push_str(endpoint.unwrap_or(""))
                    .map_err(|_| "endpoint exceeds 128 bytes")?;
                cmd.listen = listen;
            }
            ZenohCommand::Get {
                parameters,
                timeout_ms,
                ..
            } => {
                cmd.parameters
                    .push_str(parameters)
                    .map_err(|_| "parameters exceed 128 bytes")?;
                cmd.timeout_ms = timeout_ms;
            }
            ZenohCommand::Pub {
                interval_ms, count, ..
            } => {
                cmd.interval_ms = interval_ms;
                cmd.count = count;
            }
            ZenohCommand::Querier {
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
            ZenohCommand::Ping {
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
            ZenohCommand::Pong { peer_key, .. } => {
                cmd.peer_key
                    .push_str(peer_key)
                    .map_err(|_| "peer key exceeds 128 bytes")?;
            }
            ZenohCommand::PubThr { size, count, .. } => {
                cmd.size = size;
                cmd.count = count;
            }
            ZenohCommand::SubThr { count, .. } => {
                cmd.count = count;
            }
            ZenohCommand::Broker {
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
    let mut args = raw.args().args().peekable();
    if raw.name() == "zenoh" && matches!(args.peek(), Some(Ok(Arg::Value("status" | "close")))) {
        args.next();
        if args.next().is_some() {
            return Err("this command takes no arguments");
        }
        return Ok(());
    }
    for arg in args {
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
