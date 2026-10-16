# Desktop hardware companion implementation plan

## Goal and authorization

Create the functional, public GitHub starter requested in this conversation: a pnpm/Turborepo monorepo, a small Next.js/Tauri application, replaceable Groq integration, extensible device transports, and GitHub Actions. Linux is the first target; macOS and Windows must compile without Linux-only dependencies leaking into their builds. The user explicitly authorized implementation after the architectural discussion. Work proceeds in this empty directory without another approval round.

## Architecture

- `apps/desktop`: statically exported Next.js interface and the Tauri host.
- `packages/protocol`: shared TypeScript device contracts and validation.
- `packages/sdk`: desktop bridge and explicitly labeled browser simulator.
- `packages/ui`: reusable accessible controls and design tokens.
- `packages/typescript-config`: shared compiler configuration.
- `crates/core`: platform-independent commands, validation and simulator.
- `crates/providers`: provider interface, configurable Groq-compatible HTTP adapter, bounded agent loop and offline demonstration provider.
- `crates/transports`: serial USB/UART, WebSocket, BLE and Linux RFCOMM adapters.
- `crates/runtime`: device sessions, command dispatch and GUI-independent integration tests.
- `firmware`: documented reference serial firmware with host protocol tests.

## Global constraints

Use pnpm and Turborepo, Next.js static export and Tauri 2. Keep credentials in the native process environment. Never silently substitute simulation for a failed real connection. Show simulated devices explicitly. Share logical device commands across transports, with transport-specific framing. Keep Linux RFCOMM behind target configuration. The starter must be usable without hardware or API keys.

## Tasks

- [x] Establish workspace manifests, product/design context, license and public-project documentation.
- [x] Write failing protocol/core tests for invalid commands, unknown devices and state changes; implement contracts and simulator.
- [x] Write provider tests for tool execution, malformed arguments and bounded loops; implement configurable provider and agent.
- [x] Implement transport boundaries, bounded message framing and explicit platform availability; verify parsing and local WebSocket exchange.
- [x] Connect Tauri commands to the native runtime and implement the compact panel, device controls, connections and browser preview.
- [x] Add pinned GitHub Actions for JavaScript/browser verification, Rust checks and native builds on Linux, macOS and Windows; add manual artifact packaging and Dependabot.
- [x] Run checks, browser interaction and a fresh independent code review; fix material findings, document verification limits, then create and push the public repository (publication and remote CI are the final step).

## Verification

Use Vitest for protocol/SDK behavior, Rust unit and local-server integration tests for core/providers/transports, Biome and TypeScript for frontend checks, a production static build, and Playwright for the actual browser workflow. Native CI installs platform prerequisites and compiles Tauri on each OS. No hardware connection or API call is claimed verified without equipment or credentials.

## Review focus

1. Invalid servo angles or unsupported device actions must never reach a transport.
2. Provider failures and malformed tool calls must remain visible rather than becoming apparent successes.
3. A detached device, timeout or malformed frame must not hang the UI or trigger blind actuator retries.
4. Browser simulation must remain explicitly separate from native hardware control.
5. Public CI must use read-only permissions and must not require secrets to pass.
