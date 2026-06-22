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
 * Interface for objects that can receive items asynchronously.
 * Matches the `ChannelReceiver<T>` shape from `@eclipse-zenoh/zenoh-ts`.
 */
export interface ChannelReceiver<T> {
  receive(): Promise<T | null>;
  [Symbol.asyncIterator](): AsyncIterator<T>;
}

interface Waiter<T> {
  resolve: (v: T | null) => void;
  reject: (e: unknown) => void;
}

/**
 * A bounded FIFO channel that bridges WASM callbacks to async TypeScript iterators.
 *
 * Dropped (newest) on overflow — preserves oldest items like a classic FIFO queue.
 *
 * **Single-consumer.** Every `[Symbol.asyncIterator]()` call shares the same
 * underlying queue, so concurrent iterators would steal items from one another.
 * Drive a channel from a single `for await` loop (or a single `receive()` chain).
 * Exiting a `for await` loop early (via `break`/`return`/`throw`) closes the
 * channel through the iterator's `return()`, so the producer stops filling it.
 */
export class FifoChannel<T> implements ChannelReceiver<T> {
  protected _queue: T[] = [];
  protected _waiters: Array<Waiter<T>> = [];
  protected _closed = false;
  protected _failed = false;
  protected _error: unknown = undefined;

  constructor(readonly capacity: number = 256) {}

  /** Push an item into the channel (called synchronously from WASM callback). */
  push(item: T): void {
    if (this._closed) return;
    if (this._waiters.length > 0) {
      this._waiters.shift()!.resolve(item);
    } else if (this._queue.length < this.capacity) {
      this._queue.push(item);
    }
    // else: drop newest (FIFO overflow policy)
  }

  /**
   * Receive the next item, waiting asynchronously if the channel is empty.
   *
   * Resolves to `null` once the channel is closed and drained; rejects with
   * the failure reason if the channel was failed via {@link fail}.
   */
  async receive(): Promise<T | null> {
    if (this._queue.length > 0) return this._queue.shift()!;
    if (this._failed) throw this._error;
    if (this._closed) return null;
    return new Promise<T | null>((resolve, reject) => {
      this._waiters.push({ resolve, reject });
    });
  }

  /** Non-blocking peek: returns undefined if empty. */
  tryReceive(): T | undefined {
    return this._queue.shift();
  }

  /** Close the channel, causing all pending and future receives to return null. */
  close(): void {
    if (this._closed) return;
    this._closed = true;
    for (const w of this._waiters) w.resolve(null);
    this._waiters = [];
  }

  /**
   * Fail the channel with `error`. Drains nothing: pending and future
   * `receive()` calls reject with `error` instead of returning `null`, so a
   * consumer can distinguish a transport failure from normal completion.
   */
  fail(error: unknown): void {
    if (this._closed) return;
    this._closed = true;
    this._failed = true;
    this._error = error;
    for (const w of this._waiters) w.reject(error);
    this._waiters = [];
  }

  get isClosed(): boolean {
    return this._closed;
  }

  [Symbol.asyncIterator](): AsyncIterator<T> {
    // eslint-disable-next-line @typescript-eslint/no-this-alias
    const self = this;
    return {
      async next(): Promise<IteratorResult<T>> {
        const value = await self.receive();
        if (value === null)
          return { done: true, value: undefined as unknown as T };
        return { done: false, value };
      },
      // Called when the consumer exits the loop early (break/return/throw),
      // so the producer stops filling a channel nobody is reading.
      async return(): Promise<IteratorResult<T>> {
        self.close();
        return { done: true, value: undefined as unknown as T };
      },
    };
  }

  /** Closes the channel when used with `await using`. */
  [Symbol.asyncDispose](): Promise<void> {
    this.close();
    return Promise.resolve();
  }
}

/**
 * A bounded ring-buffer channel: on overflow, drops the **oldest** item to make room.
 */
export class RingChannel<T> extends FifoChannel<T> {
  override push(item: T): void {
    if (this._closed) return;
    if (this._waiters.length > 0) {
      this._waiters.shift()!.resolve(item);
    } else {
      if (this._queue.length >= this.capacity) {
        this._queue.shift(); // drop oldest
      }
      this._queue.push(item);
    }
  }
}
