# Changelog

Format: [Keep a Changelog](https://keepachangelog.com). Every entry must be
verifiable from the repository or CI.

## [Unreleased]

### Added
- Splitter contract: `create_split`, `deposit`, `claim`, `claimable`,
  `claimed`, `get_split`.
- Unit tests and a conservation property test.
- CI: rustfmt, clippy, test, docs, audit, cargo-deny, WASM build and size cap.
- Release workflow publishing WASM and SHA-256 checksums on `v*.*.*` tags.
