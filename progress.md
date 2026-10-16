# Progress

The boardmate starter is implemented. Workspace packages, native runtime, configurable providers, four host transport adapters, compact panel, MicroPython serial reference, public documentation and GitHub workflows are present. The initial public GitHub push is authorized and is the final integration step.

## Local verification

- `pnpm check`: Biome recommended rules and strict TypeScript pass.
- `pnpm test`: 17 tests pass.
- `pnpm build`: Next.js production static export succeeds.
- `PLAYWRIGHT_CHANNEL=chrome pnpm test:e2e`: 4 desktop/mobile scenarios pass, including direct controls, collapse/reopen, assistant actuation and invalid-angle rejection.
- `pnpm rust:check`: formatting and Clippy pass for GUI-independent crates.
- `pnpm rust:test`: 17 tests pass, including a local HTTP provider and real local WebSocket integration.
- Reference firmware protocol: 5 host tests pass.
- Independent review findings fixed: header drag capability, macOS Bluetooth usage description and preserving confirmed actions after provider failures. Follow-up review reported no important remaining defect.

Node 22.23.2, pnpm 11.22.0 and Rust 1.98.1 were used. Native GTK/WebKit development packages are missing locally and passwordless sudo is unavailable; native compilation is delegated to GitHub Actions with installed prerequisites. Physical boards, Bluetooth permissions and live Groq inference have not been tested. Do not infer hardware validation from host tests.
