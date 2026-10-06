# Feature status

Legend: ✅ done and tested in-repo · 🟡 written, not yet verified by CI/run · ❌ not started.
Update this file in the same PR as any change that moves a row.

| Capability | Status | Evidence |
| --- | --- | --- |
| `create_split` validation (count, shares, duplicates, sum) | 🟡 | `crates/splitter/src/tests.rs` |
| `deposit` with real SEP-41 transfer, transfer-before-state | 🟡 | `tests.rs` |
| `claim` pull payout, `NothingToClaim`, `NotRecipient` | 🟡 | `tests.rs` |
| Conservation invariant under random sequences | 🟡 | `props.rs` |
| Missing-authorization rejection | 🟡 | `tests.rs::calls_without_authorization_are_rejected` |
| Events (`split_created`, `deposited`, `claimed`) | 🟡 | emitted in `lib.rs`; not yet asserted in tests |
| CI: fmt, clippy, test, docs, audit, deny, wasm size | 🟡 | `.github/workflows/ci.yml` |
| Testnet deployment, contract ID, WASM hash, receipts | ❌ | pending |
| Re-run of demo from a clean machine | ❌ | pending |
| TypeScript bindings | ❌ | out of scope for v0.1 |
| External audit | ❌ | out of scope; never claimed |

🟡 becomes ✅ only after the first fully green CI run on `main`.
