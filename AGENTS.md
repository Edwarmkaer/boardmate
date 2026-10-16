# Project instructions

Read `PRODUCT.md`, `DESIGN.md`, `PLAN.md` and `progress.md` before continuing work. Keep this public starter small and preserve explicit simulation labels.

- JavaScript packages use pnpm and Turborepo. Rust crates share the root Cargo workspace.
- Keep credentials in Rust environment/configuration, never in `NEXT_PUBLIC_*`, frontend code, fixtures or logs.
- Commands must pass core validation before reaching hardware. Never blindly retry actuator operations.
- Core/providers must remain platform-independent. Linux-specific Bluetooth code belongs behind target configuration in transports.
- Native failures must stay failures; do not silently switch to simulated devices.
- Run `pnpm check`, `pnpm test`, `pnpm build` and the documented Rust checks before declaring completion.
- The initial repository bootstrap and public GitHub push are explicitly authorized by the user. Initial product/design files record the previously discussed compact panel; they do not imply that the user reviewed every visual token. Do not add approval ceremonies for this authorized starter.
- Later product changes should update these documents proportionally. Never commit credentials or generated build artifacts.
