#![no_std]

//! # Stellar Splitter
//!
//! A payment splitter for Soroban. A creator registers a **split**: one SEP-41
//! token and up to [`MAX_RECIPIENTS`] recipients whose shares, in basis
//! points, add up to exactly [`TOTAL_BPS`]. Anyone can deposit that token into
//! the split. Each recipient later **pulls** their share with `claim`.
//!
//! ## Accounting
//!
//! Nothing is distributed at deposit time. The contract keeps one running
//! counter per split, `total_deposited`, and one `claimed` counter per
//! recipient:
//!
//! ```text
//! entitled(r)  = floor(total_deposited * bps(r) / 10_000)
//! claimable(r) = entitled(r) - claimed(r)
//! ```
//!
//! Because entitlement is recomputed from the *cumulative* total, rounding
//! never compounds across deposits. At most `n - 1` base units of dust (for
//! `n` recipients) stay in the contract; see `docs/KNOWN-LIMITATIONS.md`.
//!
//! ## Conservation invariant
//!
//! For every split, at every reachable state:
//!
//! ```text
//! total_deposited == sum(claimed) + tokens still held for the split
//! ```
//!
//! ## Transfer-before-state
//!
//! `deposit` and `claim` move tokens **before** writing any state. A failed
//! transfer aborts the invocation and the host rolls everything back, so the
//! counters can never run ahead of the tokens.

#[cfg(test)]
extern crate std;

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Env, Vec,
};

/// Maximum recipients per split. Bounds validation cost and storage size.
pub const MAX_RECIPIENTS: u32 = 10;
/// Shares are in basis points and must add up to exactly this value.
pub const TOTAL_BPS: u32 = 10_000;

const DAY_IN_LEDGERS: u32 = 17_280;
const BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
const BUMP_THRESHOLD: u32 = BUMP_AMOUNT - DAY_IN_LEDGERS;

/// Errors returned by the splitter contract.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum SplitterError {
    /// An argument failed validation (shares, recipient count, amount).
    InvalidInput = 1,
    /// No split exists with that id.
    NotFound = 2,
    /// The caller is not a recipient of this split.
    NotRecipient = 3,
    /// The recipient has nothing to claim right now.
    NothingToClaim = 4,
    /// An arithmetic operation would overflow.
    Overflow = 5,
}

/// One recipient and their share in basis points.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recipient {
    pub address: Address,
    pub bps: u32,
}

/// Stored record for a split.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitData {
    pub creator: Address,
    pub token: Address,
    pub recipients: Vec<Recipient>,
    /// Cumulative amount ever deposited. Monotonically increasing.
    pub total_deposited: i128,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Count,
    Split(u64),
    Claimed(u64, Address),
}

mod events {
    use super::*;

    #[contractevent]
    pub struct SplitCreated {
        #[topic]
        pub split_id: u64,
        pub creator: Address,
        pub token: Address,
    }

    #[contractevent]
    pub struct Deposited {
        #[topic]
        pub split_id: u64,
        pub from: Address,
        pub amount: i128,
    }

    #[contractevent]
    pub struct Claimed {
        #[topic]
        pub split_id: u64,
        #[topic]
        pub recipient: Address,
        pub amount: i128,
    }
}

#[contract]
pub struct Splitter;

#[contractimpl]
impl Splitter {
    /// Register a new split and return its id.
    ///
    /// `creator` must authorize. Shares must be non-zero, recipients unique,
    /// the count between 1 and [`MAX_RECIPIENTS`], and shares must add up to
    /// exactly [`TOTAL_BPS`].
    pub fn create_split(
        env: Env,
        creator: Address,
        token: Address,
        recipients: Vec<Recipient>,
    ) -> Result<u64, SplitterError> {
        creator.require_auth();
        validate_recipients(&recipients)?;

        let id = env
            .storage()
            .instance()
            .get::<_, u64>(&DataKey::Count)
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(SplitterError::Overflow)?;
        env.storage().instance().set(&DataKey::Count, &id);
        env.storage()
            .instance()
            .extend_ttl(BUMP_THRESHOLD, BUMP_AMOUNT);

        let split = SplitData {
            creator: creator.clone(),
            token: token.clone(),
            recipients,
            total_deposited: 0,
        };
        save_split(&env, id, &split);

        events::SplitCreated {
            split_id: id,
            creator,
            token,
        }
        .publish(&env);
        Ok(id)
    }

    /// Deposit `amount` of the split's token from `from` into the split.
    ///
    /// `from` must authorize. Tokens are transferred first; state is written
    /// only after the transfer succeeds.
    pub fn deposit(
        env: Env,
        split_id: u64,
        from: Address,
        amount: i128,
    ) -> Result<(), SplitterError> {
        from.require_auth();
        if amount <= 0 {
            return Err(SplitterError::InvalidInput);
        }
        let mut split = load_split(&env, split_id)?;

        let new_total = split
            .total_deposited
            .checked_add(amount)
            .ok_or(SplitterError::Overflow)?;
        // Guarantee every later entitlement multiplication fits in i128.
        new_total
            .checked_mul(i128::from(TOTAL_BPS))
            .ok_or(SplitterError::Overflow)?;

        token::TokenClient::new(&env, &split.token).transfer(
            &from,
            env.current_contract_address(),
            &amount,
        );

        split.total_deposited = new_total;
        save_split(&env, split_id, &split);

        env.storage()
            .instance()
            .extend_ttl(BUMP_THRESHOLD, BUMP_AMOUNT);

        events::Deposited {
            split_id,
            from,
            amount,
        }
        .publish(&env);
        Ok(())
    }

    /// Pay `recipient` everything currently claimable and return the amount.
    ///
    /// `recipient` must authorize. Returns `NothingToClaim` if the amount is
    /// zero. Tokens are transferred first; state is written only afterwards.
    pub fn claim(env: Env, split_id: u64, recipient: Address) -> Result<i128, SplitterError> {
        recipient.require_auth();
        let split = load_split(&env, split_id)?;
        let (entitled, claimed) = entitlement_of(&env, split_id, &split, &recipient)?;
        let amount = entitled - claimed;
        if amount <= 0 {
            return Err(SplitterError::NothingToClaim);
        }

        token::TokenClient::new(&env, &split.token).transfer(
            &env.current_contract_address(),
            &recipient,
            &amount,
        );

        let key = DataKey::Claimed(split_id, recipient.clone());
        env.storage().persistent().set(&key, &entitled);
        env.storage()
            .persistent()
            .extend_ttl(&key, BUMP_THRESHOLD, BUMP_AMOUNT);
        bump_split(&env, split_id);

        env.storage()
            .instance()
            .extend_ttl(BUMP_THRESHOLD, BUMP_AMOUNT);

        events::Claimed {
            split_id,
            recipient,
            amount,
        }
        .publish(&env);
        Ok(amount)
    }

    /// Amount `recipient` could claim right now. Read-only.
    pub fn claimable(env: Env, split_id: u64, recipient: Address) -> Result<i128, SplitterError> {
        let split = load_split(&env, split_id)?;
        let (entitled, claimed) = entitlement_of(&env, split_id, &split, &recipient)?;
        Ok(entitled - claimed)
    }

    /// Amount `recipient` has already claimed. Zero for non-recipients.
    pub fn claimed(env: Env, split_id: u64, recipient: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Claimed(split_id, recipient))
            .unwrap_or(0)
    }

    /// Return the stored split record.
    pub fn get_split(env: Env, split_id: u64) -> Result<SplitData, SplitterError> {
        load_split(&env, split_id)
    }
}

fn validate_recipients(recipients: &Vec<Recipient>) -> Result<(), SplitterError> {
    let n = recipients.len();
    if n == 0 || n > MAX_RECIPIENTS {
        return Err(SplitterError::InvalidInput);
    }
    let mut sum: u32 = 0;
    for i in 0..n {
        let r = recipients.get(i).ok_or(SplitterError::InvalidInput)?;
        if r.bps == 0 {
            return Err(SplitterError::InvalidInput);
        }
        sum = sum.checked_add(r.bps).ok_or(SplitterError::InvalidInput)?;
        for j in (i + 1)..n {
            let other = recipients.get(j).ok_or(SplitterError::InvalidInput)?;
            if other.address == r.address {
                return Err(SplitterError::InvalidInput);
            }
        }
    }
    if sum != TOTAL_BPS {
        return Err(SplitterError::InvalidInput);
    }
    Ok(())
}

fn load_split(env: &Env, id: u64) -> Result<SplitData, SplitterError> {
    env.storage()
        .persistent()
        .get(&DataKey::Split(id))
        .ok_or(SplitterError::NotFound)
}

fn save_split(env: &Env, id: u64, split: &SplitData) {
    let key = DataKey::Split(id);
    env.storage().persistent().set(&key, split);
    env.storage()
        .persistent()
        .extend_ttl(&key, BUMP_THRESHOLD, BUMP_AMOUNT);
}

fn bump_split(env: &Env, id: u64) {
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Split(id), BUMP_THRESHOLD, BUMP_AMOUNT);
}

/// Returns `(entitled, claimed)` for `who`, or `NotRecipient`.
fn entitlement_of(
    env: &Env,
    id: u64,
    split: &SplitData,
    who: &Address,
) -> Result<(i128, i128), SplitterError> {
    let bps = split
        .recipients
        .iter()
        .find(|r| &r.address == who)
        .map(|r| r.bps)
        .ok_or(SplitterError::NotRecipient)?;
    let entitled = split
        .total_deposited
        .checked_mul(i128::from(bps))
        .ok_or(SplitterError::Overflow)?
        / i128::from(TOTAL_BPS);
    let claimed: i128 = env
        .storage()
        .persistent()
        .get(&DataKey::Claimed(id, who.clone()))
        .unwrap_or(0);
    Ok((entitled, claimed))
}

#[cfg(test)]
mod props;
#[cfg(test)]
mod tests;
