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
