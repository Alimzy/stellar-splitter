# Contributing

Thanks for helping. This project is small on purpose: one contract, done well.

## Setup

```bash
rustup toolchain install stable --component rustfmt clippy
rustup target add wasm32v1-none
cargo install --locked stellar-cli --version 28.0.0   # or use the prebuilt binary
make check
```

## Before opening a PR

- `make check` passes (fmt, clippy `-D warnings`, tests, WASM build).
- New behaviour has a test. Anything touching payouts needs a property test or
  an extension of `props.rs`.
- Claims in docs link evidence. If you change behaviour, update
  `docs/FEATURE-STATUS.md` and `docs/KNOWN-LIMITATIONS.md` in the same PR.
- Keep PRs focused: one issue, one PR. Reference it with `Closes #N`.
- No secrets, keys or generated files in commits.

## Issues and Drips Wave

Issues carry a complexity label: `trivial` (typos, small fixes), `medium`
(contained features, tests), `high` (new contract behaviour). Comment on an
issue to claim it and wait for assignment before starting.

## Conduct

Be kind, be specific, assume good faith.
