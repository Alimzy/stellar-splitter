# Stellar Splitter

A pro-rata **payment splitter** for Soroban. One contract, one job: take a
SEP-41 token, divide it between up to 10 recipients by share (basis points),
and let each recipient pull exactly what they are owed.

> **Status: v0.1.0, demonstrated on testnet.** Contract, tests and CI pass on main, and a testnet demo run is documented below.
> Testnet deployment and receipts are tracked in
> [docs/FEATURE-STATUS.md](docs/FEATURE-STATUS.md). Nothing in this README
> claims more than that file does.

## What it does

| Entrypoint | Auth | Effect |
| --- | --- | --- |
| `create_split(creator, token, recipients)` | creator | Registers shares. Must be 1-10 unique recipients, each share > 0, summing to exactly 10,000 bps. |
| `deposit(split_id, from, amount)` | from | Moves `amount` of the split's SEP-41 token into the contract, then bumps `total_deposited`. |
| `claim(split_id, recipient)` | recipient | Pays the recipient everything currently owed. Fails with `NothingToClaim` at zero. |
| `claimable` / `claimed` / `get_split` | none | Read-only views. |

```text
entitled(r)  = floor(total_deposited * bps(r) / 10_000)
claimable(r) = entitled(r) - claimed(r)
```

**Invariant (property-tested):** `total deposited == sum(paid out) + held by
the contract`, after every step, for random shares, deposits and claim orders.

## Design decisions

- **Pull, not push.** One bad recipient (frozen trustline, rejecting token)
  can never block anyone else's payout.
- **Cumulative accounting.** Entitlement is recomputed from the running total,
  so rounding error does not compound across deposits. At most `n - 1` base
  units of dust remain in the contract; see
  [KNOWN-LIMITATIONS](docs/KNOWN-LIMITATIONS.md).
- **Transfer before state.** A failed token transfer aborts the whole
  invocation, so counters never run ahead of the tokens.
- **Immutable splits.** Recipients and shares cannot be changed after creation.
- **Bounded.** Max 10 recipients keeps validation and storage cost fixed.
- **TTL handled.** Persistent entries are extended on every write.

## Quick start

```bash
rustup target add wasm32v1-none
cargo test --workspace --all-targets
# Needs stellar-cli 28.x (soroban-sdk 28 requires the spec-shaking build):
stellar contract build --profile release
```

Run everything CI runs: `make check`.

### Testnet demo

```bash
bash scripts/demo-testnet.sh --help
bash scripts/demo-testnet.sh --dry-run     # prints every command, runs none
bash scripts/demo-testnet.sh               # live: needs funded identities
```

The script never creates, prints or commits secret keys.

## Repository layout

```text
crates/splitter/        the contract, unit tests (tests.rs), property tests (props.rs)
docs/                   feature status, known limitations
scripts/demo-testnet.sh reproducible testnet lifecycle
.github/workflows/      ci.yml (fmt, clippy, test, docs, audit, deny, wasm size), release.yml
```

## Contributing

Scoped, labeled issues are the entry point; see
[CONTRIBUTING.md](CONTRIBUTING.md). Security reports: [SECURITY.md](SECURITY.md).

## License

MIT. See [LICENSE](LICENSE).

## Testnet deployment

Run on Stellar testnet with `scripts/demo-testnet.sh`: a 70/30 split between Bob and Carol, funded with 1 XLM (10000000 stroops) from `demo-alice`.

| Item | Value |
| --- | --- |
| Contract ID | `CA3Z56THQBYF5WFBULMNTEZPXWAOHGGYEK2AXJNLXYFWFVGKNRHCJEZ2` |
| WASM hash (sha256) | `836047395bb6d4d339c47f25a43071acbb37201a84f0f21cf4dd8a2998eff32f` |
| Deploy | [tx](https://stellar.expert/explorer/testnet/tx/c83d05da298c7e96254d541fb13e892ce834aaf02a58eaa4b40ad3f1d18b40f0) |
| create_split | [tx](https://stellar.expert/explorer/testnet/tx/365ea6d7ccdf9581dca346818b7d6bc3366db95263d9737006ddf966cfc4e2e7) |
| deposit (10000000) | [tx](https://stellar.expert/explorer/testnet/tx/a3d56a738429fe5b2d779e1acaca18da5ce0e071ffc28068c38d7693b0ae6a4c) |
| Bob claim (7000000) | [tx](https://stellar.expert/explorer/testnet/tx/6a702bef4a3ff19ba3c9e42e2b5914871151a8064e60ba243aff60dd46c7c050) |
| Carol claim (3000000) | [tx](https://stellar.expert/explorer/testnet/tx/f03b0aa0d3ec4974f25ba24e9c1b99207f32b68b2d5e305fc7eade4a4267ee94) |
| Contract balance after claims | 0 (checked with a direct `balance` query) |

This is a testnet demo run, not an audit or a mainnet deployment. A re-run from a clean machine has not been done yet.
