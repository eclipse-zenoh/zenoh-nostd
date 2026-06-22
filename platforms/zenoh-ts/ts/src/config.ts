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
/** Session configuration. */
export class Config {
    readonly locator: string;

    /**
     * @param locator - WebSocket locator for the Zenoh router.
     *   Format: `ws/<host>:<port>` e.g. `ws/127.0.0.1:7447`
     *   Defaults to `ws/127.0.0.1:7447`.
     */
    constructor(locator: string = "ws/127.0.0.1:7447") {
        this.locator = locator;
    }

    /** Parse a locator string or Config, returning a Config. */
    static from(config: Config | string | undefined): Config {
        if (!config) return new Config();
        if (typeof config === "string") return new Config(config);
        return config;
    }
}
