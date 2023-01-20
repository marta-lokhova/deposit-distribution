#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Accounts, Ledger, LedgerInfo};
use soroban_sdk::{AccountId, Env, IntoVal};

soroban_sdk::contractimport!(
    file = "target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
);

type TokenClient = Client;

fn create_token_contract(e: &Env, admin: &AccountId) -> (BytesN<32>, TokenClient) {
    e.install_contract_wasm(WASM);

    let id = e.register_contract_wasm(None, WASM);
    let token = TokenClient::new(e, &id);
    // decimals, name, symbol don't matter in tests
    token.initialize(
        &Identifier::Account(admin.clone()),
        &7u32,
        &"name".into_val(e),
        &"symbol".into_val(e),
    );
    (id, token)
}

fn create_distribution_contract(e: &Env, admin: &AccountId, token: BytesN<32>) -> DistributionContractClient {
    let distr = DistributionContractClient::new(e, e.register_contract(None, DistributionContract {}));
    distr.initialize(&Identifier::Account(admin.clone()), &200, &token);
    distr
}

struct DistributionTest {
    token_admin: AccountId,
    attendee_users: [AccountId; 3],
    token: TokenClient,
    contract: DistributionContractClient,
}

impl DistributionTest {

    fn setup() -> Self {
        let env: Env = Default::default();
        env.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 1,
            sequence_number: 10,
            network_passphrase: Default::default(),
            base_reserve: 10,
        });

        let attendee_users = [
            env.accounts().generate(),
            env.accounts().generate(),
            env.accounts().generate(),
        ];

        let token_admin = env.accounts().generate();

        let (token_id, token) = create_token_contract(&env, &token_admin);
        for attendee in attendee_users.clone() {
            token.with_source_account(&token_admin).mint(
                &Signature::Invoker,
                &0,
                &Identifier::Account(attendee.clone()),
                &1000,
            );
        }

        token.with_source_account(&token_admin).mint(
            &Signature::Invoker,
            &0,
            &Identifier::Account(token_admin.clone()),
            &1000,
        );

        let contract = create_distribution_contract(&env, &token_admin, token_id);
        DistributionTest {
            token_admin,
            attendee_users,
            token,
            contract,
        }
    }

    fn deposit(&self, attendee: &Identifier) {
        self.call_deposit(&attendee);
    }

    fn attend(&self, attendee: &Identifier) {
        self.call_attend(attendee);
    }

    fn withdraw(&self, high: u32, low: u32) -> i32 {
        self.call_withdraw(high, low)
    }

    fn call_deposit(
        &self,
        attendee: &Identifier,
    ) {
        self.contract.deposit(attendee);
    }

    fn account_id_to_identifier(&self, account_id: &AccountId) -> Identifier {
        Identifier::Account(account_id.clone())
    }

    fn call_withdraw(
        &self, high: u32, low: u32
    ) -> i32 {
        self.contract.with_source_account(&self.token_admin).withdraw(&high, &low)
    }

    fn call_attend(
        &self,
        attendee: &Identifier
    ) {
        self.contract.with_source_account(&self.token_admin).attend(attendee);
    }

    fn approve_deposit(&self, amount: u32, user: AccountId) {
        self.token
            .with_source_account(&user)
            .incr_allow(
                &Signature::Invoker,
                &0,
                &Identifier::Contract(self.contract.contract_id.clone()),
                &(amount as i128),
            )
    }

}

#[test]
#[should_panic(expected = "not authorized by admin")]
fn test_unauthorized_withdrawal() {
    let test = DistributionTest::setup();

    // Attendee can't trigger withdrawal
    test.contract.with_source_account(&test.attendee_users[0].clone()).withdraw(&5, &0);
}

#[test]
#[should_panic(expected = "not authorized by admin")]
fn test_unauthorized_attendance() {
    let test = DistributionTest::setup();

    // Attendee can't trigger attendance counting
    test.contract.with_source_account(&test.attendee_users[0].clone()).attend(&test.account_id_to_identifier(&test.attendee_users[0].clone()));
}

#[test]
#[should_panic(expected = "attendance already recorded")]
fn test_attendee_added_twice() {
    let test = DistributionTest::setup();

    test.approve_deposit(200, test.attendee_users[0].clone());

    test.deposit(&test.account_id_to_identifier(&test.attendee_users[0].clone()));
    test.attend(&test.account_id_to_identifier(&test.attendee_users[0].clone()));
    test.attend(&test.account_id_to_identifier(&test.attendee_users[0].clone()));
}

#[test]
#[should_panic(expected = "admin cannot deposit")]
fn test_admin_deposits() {
    let test = DistributionTest::setup();
    test.deposit(&test.account_id_to_identifier(&test.token_admin));
}

#[test]
#[should_panic(expected = "admin cannot attend")]
fn test_admin_attends() {
    let test = DistributionTest::setup();
    test.attend(&test.account_id_to_identifier(&test.token_admin));
}

#[test]
#[should_panic(expected = "attendee did not register")]
fn test_unregistered_attendee() {
    let test = DistributionTest::setup();

    test.attend(&test.account_id_to_identifier(&test.attendee_users[0].clone()));
}

#[test]
#[should_panic(expected = "attendee already registered")]
fn test_register_twice() {
    let test = DistributionTest::setup();

    test.approve_deposit(200, test.attendee_users[0].clone());
    test.deposit(&test.account_id_to_identifier(&test.attendee_users[0].clone()));
    test.deposit(&test.account_id_to_identifier(&test.attendee_users[0].clone()));
}

#[test]
fn test_deposit_attend_and_claim() {
    let test = DistributionTest::setup();

    test.approve_deposit(200, test.attendee_users[0].clone());
    test.approve_deposit(200, test.attendee_users[1].clone());

    // has balance
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[0])),
        1000
    );
    test.deposit(
        &test.account_id_to_identifier(&test.attendee_users[0])
    );
    test.deposit(
        &test.account_id_to_identifier(&test.attendee_users[1])
    );

    // balance decreased
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[0])),
        800
    );
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[1])),
        800
    );

    // User0 attends, but User1 doesn't
    test.attend(
        &test.account_id_to_identifier(&test.attendee_users[0])
    );

    // balance doesn't change
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[0])),
        800
    );
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[1])),
        800
    );

    // withdraw, everything goes to User1
    test.withdraw(5, 0);

    // balance doesn't change
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[0])),
        1200
    );
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[1])),
        800
    );

    // Second time withdraw should have no effect
    test.withdraw(5, 0);

    // balance doesn't change
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[0])),
        1200
    );
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[1])),
        800
    );

}

#[test]
fn test_batched_withdrawal() {
    let test = DistributionTest::setup();

    test.approve_deposit(200, test.attendee_users[0].clone());
    test.approve_deposit(200, test.attendee_users[1].clone());
    test.approve_deposit(200, test.attendee_users[2].clone());

    test.deposit(
        &test.account_id_to_identifier(&test.attendee_users[0])
    );
    test.deposit(
        &test.account_id_to_identifier(&test.attendee_users[1])
    );
    test.deposit(
        &test.account_id_to_identifier(&test.attendee_users[2])
    );

    // two attend
    test.attend(
        &test.account_id_to_identifier(&test.attendee_users[0])
    );
    test.attend(
        &test.account_id_to_identifier(&test.attendee_users[2])
    );

    // withdraw, everything goes to User1
    assert_eq!(test.withdraw(1, 0), 1);

    // balance doesn't change
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[0])),
        1100
    );
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[1])),
        800
    );
    // Haven't withdrawn for the third user
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[2])),
        800
    );

    // Second time withdraw should have no effect
    assert_eq!(test.withdraw(2, 0), 1);

    // balance doesn't change
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[0])),
        1100
    );
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[1])),
        800
    );
    // Haven't withdrawn for the third user
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[2])),
        1100
    );

    // Third withdrawal has no effect
    assert_eq!(test.withdraw(2, 0), 0);

    // balance doesn't change
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[0])),
        1100
    );
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[1])),
        800
    );
    // Haven't withdrawn for the third user
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[2])),
        1100
    );

}