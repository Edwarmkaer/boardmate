# Progress

The boardmate starter is implemented. Workspace packages, native runtime, configurable providers, four host transport adapters, compact panel, MicroPython serial reference, public documentation and GitHub workflows are present. The public repository is published at https://github.com/Edwarmkaer/boardmate on `main`.

## Local verification

- `pnpm check`: Biome recommended rules and strict TypeScript pass.
- `pnpm test`: 17 tests pass.
- `pnpm build`: Next.js production static export succeeds.
- `PLAYWRIGHT_CHANNEL=chrome pnpm test:e2e`: 4 desktop/mobile scenarios pass, including direct controls, collapse/reopen, assistant actuation and invalid-angle rejection.
- `pnpm rust:check`: formatting and Clippy pass for GUI-independent crates.
- `pnpm rust:test`: 17 tests pass, including a local HTTP provider and real local WebSocket integration.
- Reference firmware protocol: 5 host tests pass.
- Independent review findings fixed: header drag capability, macOS Bluetooth usage description and preserving confirmed actions after provider failures. Follow-up review reported no important remaining defect.

Node 22.23.2, pnpm 11.22.0 and Rust 1.98.1 were used. Native GTK/WebKit development packages are missing locally and passwordless sudo is unavailable; GitHub Actions compiled the native application successfully on Linux, macOS and Windows with installed prerequisites. Rust tests/Clippy also passed on all three OSes, and frontend/browser checks passed on GitHub. Validation run: https://github.com/Edwarmkaer/boardmate/actions/runs/38852946152. Physical boards, Bluetooth permissions and live Groq inference have not been tested. Do not infer hardware validation from host tests.

## Resource preference

The user requires consultation before resource-intensive local processes. This is recorded in `AGENTS.md`. The Next.js development server was stopped; an already-built static preview is served at http://127.0.0.1:1420 with approximately 23 MB resident server memory at the last observation. Native CI builds run on GitHub, not on the laptop.

## Linux packaging

Linux-only AppImage and `.deb` packaging completed successfully on GitHub Actions: https://github.com/Edwarmkaer/boardmate/actions/runs/38983154681. The generated packages are `boardmate_0.1.0_amd64.deb` (3.80 MiB) and `boardmate_0.1.0_amd64.AppImage` (78.96 MiB). Project checks and Rust tests ran successfully in the packaging job. The workflow retains read-only repository permissions, verifies amd64 Debian metadata and produces SHA-256 checksums. Local builds/installations are not part of this step.

Generated installers are distributed as the `boardmate-linux-x86_64` workflow artifact (14-day retention). Native startup and physical hardware behavior have not yet been verified; generation and checksum checks do not establish runtime compatibility on every Linux distribution.
