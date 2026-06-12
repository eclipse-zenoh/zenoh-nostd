export enum Priority {
    RealTime = 1,
    InteractiveHigh = 2,
    InteractiveLow = 3,
    DataHigh = 4,
    Data = 5,
    DataLow = 6,
    Background = 7,
}

export enum CongestionControl {
    Drop = 0,
    Block = 1,
}

export enum Reliability {
    BestEffort = 0,
    Reliable = 1,
}

export enum SampleKind {
    Put = 0,
    Delete = 1,
}

export enum QueryTarget {
    BestMatching = 0,
    All = 1,
    AllComplete = 2,
}

export enum ConsolidationMode {
    Auto = 0,
    None = 1,
    Monotonic = 2,
    Latest = 3,
}

export enum Locality {
    SessionLocal = 0,
    Remote = 1,
    Any = 2,
}

export enum WhatAmI {
    Router = 1,
    Peer = 2,
    Client = 4,
}

export enum ReplyKeyExpr {
    Any = 0,
    MatchingQuery = 1,
}
