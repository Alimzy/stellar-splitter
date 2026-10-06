//! Property tests for the conservation invariant.

use super::tests::Fx;
use super::*;
use proptest::prelude::*;
use std::vec::Vec as StdVec;

/// Turn arbitrary positive weights into shares that sum to exactly 10_000,
/// each at least 1.
fn normalize(weights: &[u32]) -> StdVec<u32> {
    let total: u64 = weights.iter().map(|w| u64::from(*w)).sum();
    let mut out = StdVec::new();
    let mut used: u32 = 0;
    for w in &weights[..weights.len() - 1] {
        let share = (u64::from(*w) * u64::from(TOTAL_BPS) / total) as u32;
        out.push(share);
        used += share;
    }
    out.push(TOTAL_BPS - used);
    out
}

fn weights() -> impl Strategy<Value = StdVec<u32>> {
    proptest::collection::vec(1u32..=100, 1..=(MAX_RECIPIENTS as usize))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// For random shares, random deposits and random interleaved claims:
    /// nothing is created or destroyed, nobody is paid more than their
    /// entitlement, and the dust left behind is below the recipient count.
    #[test]
    fn conservation_holds(
        w in weights(),
        deposits in proptest::collection::vec(1i128..1_000_000, 1..8),
        claim_mask in proptest::collection::vec(any::<bool>(), 1..8),
    ) {
        let shares = normalize(&w);
        prop_assert_eq!(shares.iter().sum::<u32>(), TOTAL_BPS);
        prop_assert!(shares.iter().all(|s| *s > 0));

        let fx = Fx::new();
        let c = fx.client();
        let creator = fx.addr();
        let payer = fx.addr();
        let people: StdVec<Address> = shares.iter().map(|_| fx.addr()).collect();
        let parts: StdVec<(&Address, u32)> =
            people.iter().zip(shares.iter().copied()).collect();
        let id = c.create_split(&creator, &fx.token, &fx.recipients(&parts));

        let total: i128 = deposits.iter().sum();
        fx.mint(&payer, total);

        for (i, amount) in deposits.iter().enumerate() {
            c.deposit(&id, &payer, amount);
            if claim_mask[i % claim_mask.len()] {
                for p in &people {
                    if c.claimable(&id, p) > 0 {
                        c.claim(&id, p);
                    }
                }
            }
            // Invariant after every step: deposited == paid out + held.
            let paid: i128 = people.iter().map(|p| fx.tok().balance(p)).sum();
            prop_assert_eq!(paid + fx.tok().balance(&fx.contract), total_so_far(&deposits, i));
        }

        for p in &people {
            if c.claimable(&id, p) > 0 {
                c.claim(&id, p);
            }
        }

        let paid: i128 = people.iter().map(|p| fx.tok().balance(p)).sum();
        let held = fx.tok().balance(&fx.contract);
        prop_assert_eq!(paid + held, total);
        prop_assert!(held >= 0 && held < people.len() as i128);
        for (p, bps) in people.iter().zip(shares.iter()) {
            let cap = total * i128::from(*bps) / i128::from(TOTAL_BPS);
            prop_assert_eq!(fx.tok().balance(p), cap);
        }
    }
}

fn total_so_far(deposits: &[i128], upto: usize) -> i128 {
    deposits[..=upto].iter().sum()
}
