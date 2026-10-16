# boardmate

A small desktop companion for boards, sensors and actuators. Open a floating panel, control a device directly, or ask an assistant to use its advertised capabilities. Linux comes first; the native build matrix includes macOS and Windows.

Built with **Tauri 2 + Rust**, **Next.js static export + React**, **pnpm** and **Turborepo**. Tauri is the desktop application framework; no hosted Tauri service or Node.js server is required in the packaged app.

## Try the interface

Install Node.js 22 and pnpm 11, then:

```sh
pnpm install --frozen-lockfile
pnpm dev
```

Open <http://127.0.0.1:1420>. Browser preview uses two visibly simulated boards and an offline demonstration assistant. Try “Mueve el servo a 45 grados”, toggle the light, read temperature, or collapse and reopen the panel. Browser preview cannot access native transports or call Groq.

## Run the desktop app

Install Rust stable and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS. On Debian/Ubuntu:

```sh
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential libxdo-dev libssl-dev librsvg2-dev libayatana-appindicator3-dev patchelf libudev-dev libdbus-1-dev
pnpm desktop:dev
```

macOS requires Xcode command line tools. Windows requires the MSVC C++ build tools and WebView2. The Linux Bluetooth adapters use BlueZ and require a running Bluetooth service. Your user must have permission to access the selected serial device.

The native app has an undecorated, draggable, always-on-top window, a collapsed button, and a tray menu. Closing the window hides it; use the tray's **Salir** item to quit. Window placement and always-on-top behavior depend on the desktop compositor, especially on Wayland.

## Enable Groq or another provider

Copy `.env.example` to `.env` at the repository root. The native development process discovers it through dotenv; packaged applications use their process environment. For Groq set:

```dotenv
BOARDMATE_PROVIDER=groq
BOARDMATE_MODEL=llama-3.3-70b-versatile
BOARDMATE_API_BASE_URL=https://api.groq.com/openai/v1
GROQ_API_KEY=your-own-key
```

For an API implementing the same chat-completions/tool-calling format, use `BOARDMATE_PROVIDER=compatible`, set its base URL and model, and supply `BOARDMATE_API_KEY`. HTTPS is required except for localhost development. Model availability depends on your provider. Restart the native app after changing configuration.

Keys stay in the Rust process and never enter the frontend bundle. Do not bundle a shared API key into public installers. Missing credentials produce an error; boardmate never silently switches a failed provider to demonstration mode. `BOARDMATE_PROVIDER=demo` works offline and is labeled in the interface. It is a deterministic demonstration, not an LLM.

## Hardware connections

Use **Dispositivos → Conectar placa** in the native app. A board must implement boardmate's JSON protocol; an arbitrary Arduino sketch will not respond to discovery.

| Connection | Implementation | Platform scope |
| --- | --- | --- |
| USB / UART | Serial port, USB CDC or USB-to-UART bridge | Linux, macOS, Windows |
| Wi-Fi | Persistent `ws://` or `wss://` connection | Linux, macOS, Windows |
| Bluetooth LE | Nordic UART Service GATT profile | Linux, macOS, Windows |
| Bluetooth Classic | RFCOMM socket with explicit channel | Linux only in this starter |

USB HID/vendor-specific devices need another adapter. A platform being listed means an adapter is present; physical behavior still needs testing on that OS and board. Bluetooth availability also depends on the adapter and OS permissions. Wi-Fi endpoints should be on a trusted network; use `wss://` when traffic needs encryption. Device authentication and pairing workflows are not implemented by this application.

See [protocol and firmware](docs/protocol.md) for messages, BLE UUIDs and an ESP32/Pico MicroPython serial example. Firmware for Wi-Fi, BLE and Classic is a separate next step; the host adapters are already implemented. The reference firmware has host tests but has not been flashed or physically verified in this bootstrap.

## Monorepo

```text
apps/desktop/                Next.js UI + Tauri host
packages/protocol/           TypeScript contracts and command validation
packages/sdk/                Native bridge and explicit browser simulator
packages/ui/                 Shared controls and design tokens
packages/typescript-config/  Shared strict TypeScript configuration
crates/core/                 Commands, validation, device state and simulator
crates/providers/            Provider trait, Groq-compatible HTTP, bounded agent
crates/transports/           Serial, WebSocket, BLE and Linux RFCOMM
crates/runtime/              Sessions, discovery, dispatch and integration tests
firmware/micropython/        Serial reference implementation
.github/workflows/          Checks, three-OS compilation and manual packaging
```

[Architecture](docs/architecture.md) describes extension points and execution limits. This starter keeps sessions in memory. It does not yet include persisted profiles, streaming responses, background rules, automatic reconnect, or a global shortcut. Each assistant request starts with current device capabilities rather than persistent conversation memory.

## Verify and package

```sh
pnpm check
pnpm test
pnpm build
pnpm exec playwright install chromium
pnpm test:e2e
pnpm rust:check
pnpm rust:test
python3 -m unittest discover -s firmware/micropython -p 'test_*.py'
pnpm desktop:build
```

CI runs frontend/browser checks, Rust checks on three OSes and native compilation on three OSes. The manual **Package desktop** workflow produces installers as downloadable workflow artifacts. Signing and notarization are not configured; those artifacts are development builds, not signed public releases.

Local bootstrap verification covers the production web build, browser interactions, Rust core/runtime/provider/transport tests, and reference firmware protocol tests. Local native compilation requires GTK/WebKit packages that were unavailable in the bootstrap environment; check the GitHub Actions results for native compilation. Live Groq inference and physical serial/Bluetooth hardware require separate validation.

Contributions: [CONTRIBUTING.md](CONTRIBUTING.md). License: [MIT](LICENSE).
