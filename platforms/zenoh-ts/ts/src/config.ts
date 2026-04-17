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
