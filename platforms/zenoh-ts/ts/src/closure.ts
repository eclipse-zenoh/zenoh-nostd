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
/**
 * Callback/handler types — mirrors the zenoh-ts API surface so user code can
 * reference these types when typing handlers passed to `declareSubscriber` etc.
 */

export type Handler<T> =
    | ((item: T) => void)
    | ((item: T) => Promise<void>);

/** A handler that also has an optional drop/cleanup callback. */
export interface HandlerWithDrop<T> {
    callback: Handler<T>;
    onClose?: () => void;
}
