# Known limitations

Stated up front so reviewers do not have to find them.

1. **Rounding dust.** Entitlements use floor division. With `n` recipients up
   to `n - 1` base units can remain in the contract after everyone has
   claimed. They are not redistributable in v0.1.
2. **Immutable splits.** No recipient or share changes after creation.
   Create a new split instead.
3. **One token per split.** Multi-asset splits are not supported.
4. **Open deposits.** Anyone can deposit into any split. Tokens deposited by
   mistake cannot be recovered; there is no admin or refund path by design.
5. **Token trust.** The token is assumed to be a well-behaved SEP-41 token.
   Fee-on-transfer or rebasing tokens would break the conservation invariant.
6. **Frozen or unauthorized recipients.** A recipient whose trustline is
   frozen cannot claim until it is fixed; other recipients are unaffected
   (pull model).
7. **No upgrade path.** The contract is not upgradeable in v0.1.
8. **Not audited.** No external security review has been performed. Do not
   hold funds you cannot afford to lose.
9. **License policy.** `deny.toml` checks sources and bans only. A license
   allow-list is not yet enforced in CI.
10. **Upstream advisory warning.** `cargo audit` reports RUSTSEC-2024-0436
    (`paste` is unmaintained) as a warning. It is a transitive dependency of
    `soroban-sdk` 28.0.0, not a direct dependency of this project, and no
    vulnerability is reported. It will be resolved when the SDK drops it.
10. **Upstream advisory warning.** `cargo audit` reports RUSTSEC-2024-0436
    (`paste` is unmaintained) as a warning. It is a transitive dependency of
    `soroban-sdk` 28.0.0, not a direct dependency of this project, and no
    vulnerability is reported. It will be resolved when the SDK drops it.
