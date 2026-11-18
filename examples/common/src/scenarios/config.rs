use zenoh_nostd::session::zenoh;

pub const KEY_CAPACITY: usize = 128;
pub const PAYLOAD_CAPACITY: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    Open,
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
impl Scenario {
    pub fn from_name(name: &str) -> Result<Self, &'static str> {
        match name.strip_prefix("z_").unwrap_or(name) {
            "open" => Ok(Self::Open),
            "put" => Ok(Self::Put),
            "get" => Ok(Self::Get),
            "pub" => Ok(Self::Pub),
            "sub" => Ok(Self::Sub),
            "queryable" => Ok(Self::Queryable),
            "querier" => Ok(Self::Querier),
            "ping" => Ok(Self::Ping),
            "pong" => Ok(Self::Pong),
            "pub_thr" => Ok(Self::PubThr),
            "sub_thr" => Ok(Self::SubThr),
            "broker" => Ok(Self::Broker),
            _ => Err("unknown SCENARIO"),
        }
    }
}

/// Typed scenario configuration, usable directly without any command parser.
#[derive(Clone, Debug)]
pub struct Options<'a> {
    pub scenario: Scenario,
    pub key: &'a str,
    pub peer_key: &'a str,
    pub payload: &'a [u8],
    pub parameters: &'a str,
    /// Headless pause after the final flush, before releasing the connection.
    pub close_delay_ms: u64,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    pub count: Option<u64>,
    pub size: usize,
    pub listen: bool,
    pub listen1: &'a str,
    pub listen2: &'a str,
}
impl<'a> Options<'a> {
    pub const fn new(scenario: Scenario) -> Self {
        let key = match scenario {
            Scenario::Ping | Scenario::Pong => "test/ping",
            Scenario::PubThr | Scenario::SubThr => "test/thr",
            _ => "demo/example",
        };
        Self {
            scenario,
            key,
            peer_key: "test/pong",
            payload: b"Hello, from no-std!",
            parameters: "",
            close_delay_ms: 2000,
            interval_ms: 1000,
            timeout_ms: 2000,
            count: if matches!(scenario, Scenario::Ping) {
                Some(100)
            } else {
                None
            },
            size: 8,
            listen: false,
            listen1: crate::BROKER_LISTEN_1,
            listen2: crate::BROKER_LISTEN_2,
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if !matches!(self.scenario, Scenario::Open | Scenario::Broker) {
            if self.key.len() > KEY_CAPACITY {
                return Err("key exceeds 128 bytes");
            }
            zenoh::keyexpr::new(self.key).map_err(|_| "invalid key expression")?;
            if matches!(
                self.scenario,
                Scenario::Put | Scenario::Pub | Scenario::PubThr | Scenario::Ping
            ) && self.key.contains('*')
            {
                return Err("publishing key must not contain wildcards");
            }
        }
        if matches!(self.scenario, Scenario::Ping | Scenario::Pong) {
            if self.peer_key.len() > KEY_CAPACITY {
                return Err("peer key exceeds 128 bytes");
            }
            zenoh::keyexpr::new(self.peer_key).map_err(|_| "invalid peer key")?;
            if self.peer_key.contains('*') {
                return Err("peer publishing key must not contain wildcards");
            }
        }
        if self.payload.len() > PAYLOAD_CAPACITY {
            return Err("payload exceeds 256 bytes");
        }
        if self.parameters.len() > 128 {
            return Err("parameters exceed 128 bytes");
        }
        if self.close_delay_ms > 86_400_000 {
            return Err("close delay out of range (0..86400000)");
        }
        for n in [Some(self.interval_ms), Some(self.timeout_ms), self.count]
            .into_iter()
            .flatten()
        {
            if n == 0 || n > 86_400_000 {
                return Err("numeric argument out of range (1..86400000)");
            }
        }
        if self.size == 0 || self.size > PAYLOAD_CAPACITY {
            return Err("payload size out of range (1..256)");
        }
        Ok(())
    }
    /// Read typed startup settings. Native applications can supply runtime values;
    /// embedded/browser applications use `compiled_setting` through `configured`.
    pub fn from_lookup(
        read: impl Fn(&'static str) -> Option<&'a str>,
    ) -> Result<Self, &'static str> {
        fn number(value: &str) -> Result<u64, &'static str> {
            value
                .parse()
                .map_err(|_| "expected an unsigned integer in scenario settings")
        }
        let mut options = Self::new(Scenario::from_name(read("SCENARIO").unwrap_or("pub"))?);
        if let Some(value) = read("KEY_EXPR") {
            options.key = value;
        }
        if let Some(value) = read("PEER_KEY") {
            options.peer_key = value;
        }
        if let Some(value) = read("VALUE") {
            options.payload = value.as_bytes();
        }
        if let Some(value) = read("PARAMETERS") {
            options.parameters = value;
        }
        if let Some(value) = read("CLOSE_DELAY_MS") {
            options.close_delay_ms = number(value)?;
        }
        if let Some(value) = read("INTERVAL_MS") {
            options.interval_ms = number(value)?;
        }
        if let Some(value) = read("TIMEOUT_MS") {
            options.timeout_ms = number(value)?;
        }
        if let Some(value) = read("COUNT") {
            options.count = Some(number(value)?);
        }
        if let Some(value) = read("PAYLOAD") {
            options.size =
                usize::try_from(number(value)?).map_err(|_| "payload size out of range")?;
        }
        if let Some(value) = read("LISTEN") {
            options.listen = match value {
                "0" | "false" => false,
                "1" | "true" => true,
                _ => return Err("LISTEN must be 0, 1, false or true"),
            };
        }
        if let Some(value) = read("BROKER_LISTEN_1") {
            options.listen1 = value;
        }
        if let Some(value) = read("BROKER_LISTEN_2") {
            options.listen2 = value;
        }
        options.validate()?;
        Ok(options)
    }
}

pub const SETTING_NAMES: [&str; 13] = [
    "SCENARIO",
    "KEY_EXPR",
    "PEER_KEY",
    "VALUE",
    "PARAMETERS",
    "CLOSE_DELAY_MS",
    "INTERVAL_MS",
    "TIMEOUT_MS",
    "COUNT",
    "PAYLOAD",
    "LISTEN",
    "BROKER_LISTEN_1",
    "BROKER_LISTEN_2",
];
pub fn compiled_setting(name: &str) -> Option<&'static str> {
    match name {
        "SCENARIO" => option_env!("SCENARIO"),
        "KEY_EXPR" => option_env!("KEY_EXPR"),
        "PEER_KEY" => option_env!("PEER_KEY"),
        "VALUE" => option_env!("VALUE"),
        "PARAMETERS" => option_env!("PARAMETERS"),
        "CLOSE_DELAY_MS" => option_env!("CLOSE_DELAY_MS"),
        "INTERVAL_MS" => option_env!("INTERVAL_MS"),
        "TIMEOUT_MS" => option_env!("TIMEOUT_MS"),
        "COUNT" => option_env!("COUNT"),
        "PAYLOAD" => option_env!("PAYLOAD"),
        "LISTEN" => option_env!("LISTEN"),
        "BROKER_LISTEN_1" => option_env!("BROKER_LISTEN_1"),
        "BROKER_LISTEN_2" => option_env!("BROKER_LISTEN_2"),
        _ => None,
    }
}
pub fn configured() -> Result<Options<'static>, &'static str> {
    Options::from_lookup(compiled_setting)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configuration_is_typed_and_cli_independent() {
        let o = Options::from_lookup(|name| match name {
            "SCENARIO" => Some("z_pub"),
            "KEY_EXPR" => Some("sensors/a"),
            "COUNT" => Some("3"),
            "VALUE" => Some("hello world"),
            _ => None,
        })
        .unwrap();
        assert_eq!(o.scenario, Scenario::Pub);
        assert_eq!(o.key, "sensors/a");
        assert_eq!(o.count, Some(3));
        assert_eq!(o.payload, b"hello world");
        for (key, value) in [
            ("SCENARIO", "unknown"),
            ("COUNT", "0"),
            ("PAYLOAD", "257"),
            ("KEY_EXPR", "a/**"),
            ("LISTEN", "maybe"),
            ("CLOSE_DELAY_MS", "86400001"),
        ] {
            assert!(
                Options::from_lookup(|name| if name == key { Some(value) } else { None }).is_err()
            );
        }
        assert_eq!(
            Options::from_lookup(|name| (name == "CLOSE_DELAY_MS").then_some("0"))
                .unwrap()
                .close_delay_ms,
            0
        );
        let mut o = Options::new(Scenario::Ping);
        o.size = 0;
        assert!(o.validate().is_err());
    }
}
