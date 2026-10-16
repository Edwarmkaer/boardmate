# Contributing

Use Node.js 22, pnpm 11 and Rust stable. Start with the browser demo if you do not have hardware. Read `docs/architecture.md` before extending the device or provider model.

Run `pnpm check`, `pnpm test`, `pnpm build`, `pnpm rust:check` and `pnpm rust:test`. For interface behavior, install Playwright Chromium and run `pnpm test:e2e`. Reference protocol changes also require `python3 -m unittest discover -s firmware/micropython -p 'test_*.py'`.

Keep changes small and describe behavior, validation and any physical hardware used. Do not claim hardware or platform verification from compilation alone. Add behavior tests when changing protocol validation, execution semantics or transport framing. Update both TypeScript and Rust contracts when changing the wire format.

Never commit credentials, `.env` files, build output or locally paired device details. Avoid logging provider request headers. New transports must bound reads, expose errors and avoid blind retries of actuator operations. New providers should implement `LlmProvider` rather than moving credentials into the frontend.

The CI and manual packaging workflows use read-only repository permissions. Public release signing is not configured yet. A passing build is not a signed installer release.
