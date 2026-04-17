/**
 * Interface for objects that can receive items asynchronously.
 * Matches the `ChannelReceiver<T>` shape from `@eclipse-zenoh/zenoh-ts`.
 */
export interface ChannelReceiver<T> {
    receive(): Promise<T | null>;
    [Symbol.asyncIterator](): AsyncIterator<T>;
}

/**
 * A bounded FIFO channel that bridges WASM callbacks to async TypeScript iterators.
 *
 * Dropped (newest) on overflow — preserves oldest items like a classic FIFO queue.
 */
export class FifoChannel<T> implements ChannelReceiver<T> {
    protected _queue: T[] = [];
    protected _waiters: Array<(v: T | null) => void> = [];
    protected _closed = false;

    constructor(readonly capacity: number = 256) {}

    /** Push an item into the channel (called synchronously from WASM callback). */
    push(item: T): void {
        if (this._closed) return;
        if (this._waiters.length > 0) {
            this._waiters.shift()!(item);
        } else if (this._queue.length < this.capacity) {
            this._queue.push(item);
        }
        // else: drop newest (FIFO overflow policy)
    }

    /** Receive the next item, waiting asynchronously if the channel is empty. */
    async receive(): Promise<T | null> {
        if (this._queue.length > 0) return this._queue.shift()!;
        if (this._closed) return null;
        return new Promise<T | null>((resolve) => {
            this._waiters.push(resolve);
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
        for (const w of this._waiters) w(null);
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
                if (value === null) return { done: true, value: undefined as unknown as T };
                return { done: false, value };
            },
        };
    }
}

/**
 * A bounded ring-buffer channel: on overflow, drops the **oldest** item to make room.
 */
export class RingChannel<T> extends FifoChannel<T> {
    override push(item: T): void {
        if (this._closed) return;
        if (this._waiters.length > 0) {
            this._waiters.shift()!(item);
        } else {
            if (this._queue.length >= this.capacity) {
                this._queue.shift(); // drop oldest
            }
            this._queue.push(item);
        }
    }
}
