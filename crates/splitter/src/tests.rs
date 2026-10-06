use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{Client as TokenClient, StellarAssetClient};
use soroban_sdk::{Env, Vec};

pub struct Fx {
    pub env: Env,
    pub token: Address,
    pub contract: Address,
}

impl Fx {
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract_v2(admin).address();
        let contract = env.register(Splitter, ());
        Self {
            env,
            token,
            contract,
        }
    }
    pub fn client(&self) -> SplitterClient<'_> {
        SplitterClient::new(&self.env, &self.contract)
    }
    pub fn tok(&self) -> TokenClient<'_> {
        TokenClient::new(&self.env, &self.token)
    }
    pub fn mint(&self, to: &Address, amount: i128) {
        StellarAssetClient::new(&self.env, &self.token).mint(to, &amount);
    }
    pub fn addr(&self) -> Address {
        Address::generate(&self.env)
    }
    pub fn recipients(&self, parts: &[(&Address, u32)]) -> Vec<Recipient> {
        let mut v = Vec::new(&self.env);
        for (a, bps) in parts {
            v.push_back(Recipient {
                address: (*a).clone(),
                bps: *bps,
            });
        }
        v
    }
}

#[test]
fn create_split_stores_record_and_assigns_ids() {
    let fx = Fx::new();
    let c = fx.client();
    let (creator, a, b) = (fx.addr(), fx.addr(), fx.addr());
    let id1 = c.create_split(
        &creator,
        &fx.token,
        &fx.recipients(&[(&a, 7_000), (&b, 3_000)]),
    );
    let id2 = c.create_split(&creator, &fx.token, &fx.recipients(&[(&a, 10_000)]));
    assert_eq!((id1, id2), (1, 2));
    let s = c.get_split(&id1);
    assert_eq!(s.total_deposited, 0);
    assert_eq!(s.recipients.len(), 2);
    assert_eq!(s.creator, creator);
}

#[test]
fn deposit_then_claim_pays_pro_rata_and_empties_contract() {
    let fx = Fx::new();
    let c = fx.client();
    let (creator, payer, a, b) = (fx.addr(), fx.addr(), fx.addr(), fx.addr());
    fx.mint(&payer, 1_000);
    let id = c.create_split(
        &creator,
        &fx.token,
        &fx.recipients(&[(&a, 7_000), (&b, 3_000)]),
    );
    c.deposit(&id, &payer, &1_000);
    assert_eq!(fx.tok().balance(&fx.contract), 1_000);
    assert_eq!(c.claimable(&id, &a), 700);
    assert_eq!(c.claim(&id, &a), 700);
    assert_eq!(c.claim(&id, &b), 300);
    assert_eq!(fx.tok().balance(&a), 700);
    assert_eq!(fx.tok().balance(&b), 300);
    assert_eq!(fx.tok().balance(&fx.contract), 0);
    assert_eq!(c.claimed(&id, &a), 700);
}

#[test]
fn second_claim_with_no_new_deposit_is_rejected() {
    let fx = Fx::new();
    let c = fx.client();
    let (creator, payer, a) = (fx.addr(), fx.addr(), fx.addr());
    fx.mint(&payer, 100);
    let id = c.create_split(&creator, &fx.token, &fx.recipients(&[(&a, 10_000)]));
    c.deposit(&id, &payer, &100);
    c.claim(&id, &a);
    assert_eq!(c.try_claim(&id, &a), Err(Ok(SplitterError::NothingToClaim)));
}

#[test]
fn later_deposits_become_claimable_incrementally() {
    let fx = Fx::new();
    let c = fx.client();
    let (creator, payer, a, b) = (fx.addr(), fx.addr(), fx.addr(), fx.addr());
    fx.mint(&payer, 3_000);
    let id = c.create_split(
        &creator,
        &fx.token,
        &fx.recipients(&[(&a, 5_000), (&b, 5_000)]),
    );
    c.deposit(&id, &payer, &1_000);
    c.claim(&id, &a);
    c.deposit(&id, &payer, &2_000);
    assert_eq!(c.claimable(&id, &a), 1_000);
    assert_eq!(c.claimable(&id, &b), 1_500);
    c.claim(&id, &a);
    c.claim(&id, &b);
    assert_eq!(fx.tok().balance(&a), 1_500);
    assert_eq!(fx.tok().balance(&b), 1_500);
    assert_eq!(fx.tok().balance(&fx.contract), 0);
}

#[test]
fn rounding_dust_stays_in_contract_and_is_conserved() {
    let fx = Fx::new();
    let c = fx.client();
    let (creator, payer) = (fx.addr(), fx.addr());
    let (a, b, d) = (fx.addr(), fx.addr(), fx.addr());
    fx.mint(&payer, 10);
    let id = c.create_split(
        &creator,
        &fx.token,
        &fx.recipients(&[(&a, 3_333), (&b, 3_333), (&d, 3_334)]),
    );
    c.deposit(&id, &payer, &10);
    let paid = c.claim(&id, &a) + c.claim(&id, &b) + c.claim(&id, &d);
    assert_eq!(paid, 9);
    assert_eq!(fx.tok().balance(&fx.contract), 1);
    assert_eq!(paid + fx.tok().balance(&fx.contract), 10);
}

#[test]
fn non_recipient_cannot_claim() {
    let fx = Fx::new();
    let c = fx.client();
    let (creator, payer, a, outsider) = (fx.addr(), fx.addr(), fx.addr(), fx.addr());
    fx.mint(&payer, 100);
    let id = c.create_split(&creator, &fx.token, &fx.recipients(&[(&a, 10_000)]));
    c.deposit(&id, &payer, &100);
    assert_eq!(
        c.try_claim(&id, &outsider),
        Err(Ok(SplitterError::NotRecipient))
    );
}

#[test]
fn invalid_recipient_sets_are_rejected() {
    let fx = Fx::new();
    let c = fx.client();
    let (creator, a, b) = (fx.addr(), fx.addr(), fx.addr());
    let bad = |parts: &[(&Address, u32)]| {
        c.try_create_split(&creator, &fx.token, &fx.recipients(parts))
            == Err(Ok(SplitterError::InvalidInput))
    };
    assert!(bad(&[]), "empty");
    assert!(bad(&[(&a, 5_000), (&b, 4_999)]), "sum below 10000");
    assert!(bad(&[(&a, 5_000), (&b, 5_001)]), "sum above 10000");
    assert!(bad(&[(&a, 10_000), (&b, 0)]), "zero share");
    assert!(bad(&[(&a, 5_000), (&a, 5_000)]), "duplicate recipient");
}

#[test]
fn more_than_max_recipients_is_rejected() {
    let fx = Fx::new();
    let c = fx.client();
    let creator = fx.addr();
    let mut v = Vec::new(&fx.env);
    for _ in 0..(MAX_RECIPIENTS + 1) {
        v.push_back(Recipient {
            address: fx.addr(),
            bps: 1,
        });
    }
    assert_eq!(
        c.try_create_split(&creator, &fx.token, &v),
        Err(Ok(SplitterError::InvalidInput))
    );
}

#[test]
fn max_recipients_is_accepted() {
    let fx = Fx::new();
    let c = fx.client();
    let creator = fx.addr();
    let mut v = Vec::new(&fx.env);
    for _ in 0..MAX_RECIPIENTS {
        v.push_back(Recipient {
            address: fx.addr(),
            bps: TOTAL_BPS / MAX_RECIPIENTS,
        });
    }
    assert_eq!(c.create_split(&creator, &fx.token, &v), 1);
}

#[test]
fn deposit_rejects_non_positive_amount_and_unknown_split() {
    let fx = Fx::new();
    let c = fx.client();
    let (creator, payer, a) = (fx.addr(), fx.addr(), fx.addr());
    let id = c.create_split(&creator, &fx.token, &fx.recipients(&[(&a, 10_000)]));
    assert_eq!(
        c.try_deposit(&id, &payer, &0),
        Err(Ok(SplitterError::InvalidInput))
    );
    assert_eq!(
        c.try_deposit(&id, &payer, &-5),
        Err(Ok(SplitterError::InvalidInput))
    );
    assert_eq!(
        c.try_deposit(&99, &payer, &1),
        Err(Ok(SplitterError::NotFound))
    );
}

#[test]
fn deposit_without_balance_leaves_state_untouched() {
    let fx = Fx::new();
    let c = fx.client();
    let (creator, payer, a) = (fx.addr(), fx.addr(), fx.addr());
    let id = c.create_split(&creator, &fx.token, &fx.recipients(&[(&a, 10_000)]));
    assert!(c.try_deposit(&id, &payer, &500).is_err());
    assert_eq!(c.get_split(&id).total_deposited, 0);
}

#[test]
fn calls_without_authorization_are_rejected() {
    // Deliberately no `mock_all_auths`: every entrypoint that calls
    // `require_auth` must fail when no signature is provided.
    let env = Env::default();
    let admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(admin).address();
    let contract = env.register(Splitter, ());
    let c = SplitterClient::new(&env, &contract);
    let (creator, a) = (Address::generate(&env), Address::generate(&env));
    let mut v = Vec::new(&env);
    v.push_back(Recipient {
        address: a,
        bps: TOTAL_BPS,
    });
    assert!(c.try_create_split(&creator, &token, &v).is_err());
}
