# Security

This is an early hardware-control starter. Run it with boards and endpoints you trust. Device descriptions are validated but are not authenticated identities. Pair Bluetooth devices through the operating system; use secure WebSocket endpoints when appropriate.

API keys belong to each user's native environment. Never distribute a shared key in an installer, post keys in issues, or add `NEXT_PUBLIC_` secrets. The repository ignores local environment files.

For a security-sensitive issue, avoid publishing credentials or live device endpoints. Use GitHub private vulnerability reporting if enabled; otherwise contact the repository owner privately before disclosing exploit details. Routine defects can use a public issue with sanitized reproduction steps.
