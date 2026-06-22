//
// Copyright (c) 2026 Angelo Corsaro
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   Angelo Corsaro, <kydos@protonmail.com>
//
//
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
