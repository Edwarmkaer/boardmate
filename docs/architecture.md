# Architecture

The frontend is a statically exported Next.js application rendered in the OS webview. Next.js supplies React development tooling and routing; there are no API routes, server actions, SSR, or production Node.js process. Tauri hosts the webview and exposes a small command bridge to Rust.

`packages/sdk` selects the native bridge when Tauri is present. Browser preview explicitly instantiates a simulator. A rejected native invocation is returned as an error and never replaced with simulated success. Credentials and outbound provider traffic belong to Rust.

## Boundaries

- **Core** owns device identity, capabilities, state, commands and validation. Supported actions currently are `move_servo`, `set_light` and `read_temperature`.
- **Providers** depend on `LlmProvider` and `DeviceExecutor` traits. The HTTP implementation accepts configurable base URL, model and credentials. A different API format needs another trait implementation, without changing hardware transports.
- **Transports** exchange protocol frames. Serial and Linux RFCOMM use newline-delimited JSON; WebSocket uses one JSON message per request/reply; BLE uses newline-delimited JSON fragmented into small UART writes.
- **Runtime** discovers capabilities, manages connections, validates commands and dispatches them. The Tauri host delegates to this crate so integration tests do not require a GUI toolkit.

The assistant receives tools derived from currently advertised device capabilities. It must name a device; Rust verifies the device, action and arguments again before touching a transport. Direct UI controls follow the same validation path.

## Responsiveness and execution limits

I/O runs asynchronously. Connections persist between requests. The UI uses explicit pending states, limits the visible conversation to 30 messages and keeps no growing telemetry buffer. There is one active agent request and one connection setup at a time, with a maximum of 16 connected boards. Each board has one in-flight exchange; competing operations return a busy error rather than building an unbounded queue.

The agent allows six model rounds, at most four tool calls per round and a 4,000-byte input. Each model round has a 25-second deadline; HTTP requests have their own shorter deadline. Repeated tool call IDs are rejected. Provider response bodies and transport frames are bounded. Board description and command exchanges have six-second deadlines. BLE initialization uses bounded operations and attempts to stop scanning/disconnect after failure.

Completed device replies survive a later provider error. The response carries partial results and an explicit error, so the interface never hides confirmed physical actions behind a generic failed chat. A timeout does not establish whether an actuator moved. There are no blind command retries or automatic reconnects. The reference firmware deduplicates its most recent 16 request IDs; this is a bounded cache, not an exactly-once guarantee across restarts.

## Platform scope and remaining work

Linux RFCOMM and its dependencies are target-gated. BLE uses btleplug's native platform adapters. Serial and WebSocket share logical device messages across OSes. macOS includes the Bluetooth usage description; system permission prompts and pairing still need physical verification. On macOS and Windows, an OS-managed Bluetooth serial port may be usable through the serial adapter, but there is no dedicated Classic discovery adapter.

Add future USB HID support as a `DeviceTransport`, future board roles as validated capabilities, and future providers through `LlmProvider`. Keep platform-specific code out of core/providers. Persistence, secure credential storage, streaming, device authentication, automatic discovery on startup and firmware provisioning are intentionally future work.
