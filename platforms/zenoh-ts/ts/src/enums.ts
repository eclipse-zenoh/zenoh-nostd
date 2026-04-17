export enum Priority {
    RealTime = 1,
    REAL_TIME = 1,
    InteractiveHigh = 2,
    INTERACTIVE_HIGH = 2,
    InteractiveLow = 3,
    INTERACTIVE_LOW = 3,
    DataHigh = 4,
    DATA_HIGH = 4,
    Data = 5,
    DATA = 5,
    DataLow = 6,
    DATA_LOW = 6,
    Background = 7,
    BACKGROUND = 7,
}

export enum CongestionControl {
    Drop = 0,
    DROP = 0,
    Block = 1,
    BLOCK = 1,
}

export enum Reliability {
    BestEffort = 0,
    BEST_EFFORT = 0,
    Reliable = 1,
    RELIABLE = 1,
}

export enum SampleKind {
    Put = 0,
    PUT = 0,
    Delete = 1,
    DELETE = 1,
}

export enum QueryTarget {
    BestMatching = 0,
    BEST_MATCHING = 0,
    All = 1,
    ALL = 1,
    AllComplete = 2,
    ALL_COMPLETE = 2,
}

export enum ConsolidationMode {
    Auto = 0,
    AUTO = 0,
    None = 1,
    NONE = 1,
    Monotonic = 2,
    MONOTONIC = 2,
    Latest = 3,
    LATEST = 3,
}

export enum Locality {
    SessionLocal = 0,
    SESSION_LOCAL = 0,
    Remote = 1,
    REMOTE = 1,
    Any = 2,
    ANY = 2,
}

export enum WhatAmI {
    Router = 1,
    ROUTER = 1,
    Peer = 2,
    PEER = 2,
    Client = 4,
    CLIENT = 4,
}

export enum ReplyKeyExpr {
    Any = 0,
    ANY = 0,
    MatchingQuery = 1,
    MATCHING_QUERY = 1,
}
