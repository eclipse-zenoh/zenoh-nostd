// embedded-cli has no alias attribute. Generate both spellings and their
// conversion from one schema so parsing, defaults, help, and completion agree.
// Capture field types as tokens so the derive can recognize Option and bool.
macro_rules! zenoh_commands {
    ($command_lifetime:lifetime; $(
        $(#[$variant_attr:meta])*
        $variant:ident as $legacy:literal $({
            $($(#[$field_attr:meta])* $field:ident : [$($ty:tt)+]),* $(,)?
        })?
    ),* $(,)?) => {
        /// Zenoh operations to mount under an application's command group.
        ///
        /// The application validates arguments, chooses the operations its
        /// worker supports, and executes them against its session.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, embedded_cli::Command)]
        pub enum ZenohCommand<$command_lifetime> {
            $(
                $(#[$variant_attr])*
                $variant $({$($(#[$field_attr])* $field: $($ty)+),*})?,
            )*
        }

        /// Compatibility spellings used by the existing Zenoh examples.
        #[doc(hidden)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, embedded_cli::Command)]
        pub enum LegacyZenohCommand<$command_lifetime> {
            $(
                $(#[$variant_attr])*
                #[command(name = $legacy)]
                $variant $({$($(#[$field_attr])* $field: $($ty)+),*})?,
            )*
        }

        impl<$command_lifetime> From<LegacyZenohCommand<$command_lifetime>> for ZenohCommand<$command_lifetime> {
            fn from(command: LegacyZenohCommand<$command_lifetime>) -> Self {
                match command {
                    $(
                        LegacyZenohCommand::$variant $({$($field),*})? =>
                            Self::$variant $({$($field),*})?,
                    )*
                }
            }
        }
    };
}

zenoh_commands! {
    'a;
    /// Show connection state, capacities and dropped output.
    Status as "status",
    /// Open a session using the default or supplied endpoint.
    Open as "z_open" {
        /// Endpoint to connect to or listen on.
        endpoint: [Option<&'a str>],
        /// Accept an incoming connection instead of connecting.
        #[arg(long)]
        listen: [bool],
    },
    /// Close the session and release its resources.
    Close as "z_close",
    /// Publish one value.
    Put as "z_put" {
        /// Publication key expression.
        key: [&'a str],
        /// Text to publish as the payload.
        payload: [&'a str],
    },
    /// Query values until finalization or timeout.
    Get as "z_get" {
        /// Query key expression.
        key: [&'a str],
        /// Parameters to include in the selector.
        #[arg(long, default_value = "")]
        parameters: [&'a str],
        /// Timeout for each query, in milliseconds.
        #[arg(long, default_value_t = 2000)]
        timeout_ms: [u64],
    },
    /// Publish repeatedly; Ctrl-C stops an unbounded run.
    Pub as "z_pub" {
        /// Publication key expression.
        key: [&'a str],
        /// Text to publish each time.
        payload: [&'a str],
        /// Time between publications, in milliseconds.
        #[arg(long, default_value_t = 1000)]
        interval_ms: [u64],
        /// Stop after this many publications.
        #[arg(long)]
        count: [Option<u64>],
    },
    /// Receive matching samples until Ctrl-C.
    Sub as "z_sub" {
        /// Subscription key expression.
        key: [&'a str],
    },
    /// Answer queries with the supplied payload until Ctrl-C.
    Queryable as "z_queryable" {
        /// Queryable key expression.
        key: [&'a str],
        /// Text to return in replies.
        payload: [&'a str],
    },
    /// Repeat queries through a reusable querier.
    Querier as "z_querier" {
        /// Query key expression.
        key: [&'a str],
        /// Parameters to include in the selector.
        #[arg(long, default_value = "")]
        parameters: [&'a str],
        /// Timeout for each query, in milliseconds.
        #[arg(long, default_value_t = 2000)]
        timeout_ms: [u64],
        /// Time between queries, in milliseconds.
        #[arg(long, default_value_t = 1000)]
        interval_ms: [u64],
        /// Stop after this many queries.
        #[arg(long)]
        count: [Option<u64>],
    },
    /// Publish pings and measure reply latency.
    Ping as "z_ping" {
        /// Key on which to send pings.
        #[arg(default_value = "test/ping")]
        key: [&'a str],
        /// Key on which to receive replies.
        #[arg(long, default_value = "test/pong")]
        peer_key: [&'a str],
        /// Payload size in bytes.
        #[arg(long, default_value_t = 8)]
        size: [usize],
        /// Number of pings to send.
        #[arg(long, default_value_t = 100)]
        count: [u64],
        /// Timeout for each reply, in milliseconds.
        #[arg(long, default_value_t = 2000)]
        timeout_ms: [u64],
    },
    /// Echo samples to the peer key until Ctrl-C.
    Pong as "z_pong" {
        /// Key on which to receive pings.
        #[arg(default_value = "test/ping")]
        key: [&'a str],
        /// Key on which to send replies.
        #[arg(long, default_value = "test/pong")]
        peer_key: [&'a str],
    },
    /// Publish throughput traffic; Ctrl-C stops an unbounded run.
    PubThr as "z_pub_thr" {
        /// Publication key expression.
        #[arg(default_value = "test/thr")]
        key: [&'a str],
        /// Payload size in bytes.
        #[arg(long, default_value_t = 8)]
        size: [usize],
        /// Stop after this many publications.
        #[arg(long)]
        count: [Option<u64>],
    },
    /// Count received throughput traffic until Ctrl-C or the count limit.
    SubThr as "z_sub_thr" {
        /// Subscription key expression.
        #[arg(default_value = "test/thr")]
        key: [&'a str],
        /// Stop after this many samples.
        #[arg(long)]
        count: [Option<u64>],
    },
    /// Run an exclusive broker; requires alloc and listening support.
    Broker as "z_broker" {
        /// Optional upstream endpoint.
        upstream: [Option<&'a str>],
        /// First listener endpoint.
        #[arg(long = "listen-1", default_value = "")]
        listen1: [&'a str],
        /// Second listener endpoint.
        #[arg(long = "listen-2", default_value = "")]
        listen2: [&'a str],
    },
}
