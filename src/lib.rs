#![no_std]

use soroban_sdk::{contractimpl, contracttype, BytesN, Env, Vec};

mod token {
    soroban_sdk::contractimport!(file = "soroban_token_spec.wasm");
}

use token::{Identifier, Signature};

#[derive(Clone)]
#[contracttype]
pub struct Attendee {
    pub fee: u64,
    pub attended: bool,
}

// TODO: add pricing tiers (can be set by admin)
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Attendees,
    Price,
    Token
}

pub struct DistributionContract;

fn get_attendees(e: &Env) -> Vec<Identifier> {
    e.storage().get_unchecked(DataKey::Attendees).unwrap()
}

fn get_price(e: &Env) -> i128 {
    e.storage().get_unchecked(DataKey::Price).unwrap()
}

fn get_token(e: &Env) -> BytesN<32> {
    e.storage().get_unchecked(DataKey::Token).unwrap()
}

fn has_administrator(e: &Env) -> bool {
    let key = DataKey::Admin;
    e.storage().has(key)
}

fn read_administrator(e: &Env) -> Identifier {
    let key = DataKey::Admin;
    e.storage().get_unchecked(key).unwrap()
}

fn write_administrator(e: &Env, id: Identifier) {
    let key = DataKey::Admin;
    e.storage().set(key, id);
}

pub fn check_admin(e: &Env, auth_id: &Identifier) {
    if *auth_id != read_administrator(e) {
        panic!("not authorized by admin")
    }
}

#[contractimpl]
impl DistributionContract {

    pub fn initialize(
        e: Env,
        admin: Identifier,
        price: i128,
        token: BytesN<32>
    ) {
        if has_administrator(&e) {
            panic!("admin is already set");
        }

        write_administrator(&e, admin);

        let v = Vec::<Identifier>::new(&e);
        e.storage().set(DataKey::Attendees, v);
        e.storage().set(DataKey::Price, price);
        e.storage().set(DataKey::Token, token);
    }

     // Need to store depositors 
     // Can't pay twice 
    pub fn deposit(
        env: Env,
        attendee: Identifier
    ) {
        let price = get_price(&env);
        let token = get_token(&env);
        // Transfer token to this contract address.
        transfer_from_account_to_contract(&env, &token, &attendee.into(), &price);
    }
    
    pub fn attend(
        env: Env,
        attendee: Identifier
    ) {
        check_admin(&env, &env.invoker().into());

        // Store actual attendees on chain
        let mut attendees: Vec<Identifier> = get_attendees(&env);

        if attendees.contains(&attendee) {
            panic!("attendant already recorded");
        }

        attendees.push_back(attendee);
        env.storage().set(
            DataKey::Attendees,
            attendees
        )
    }

    // Distribute the money to everyone
    pub fn withdraw(
        env: Env,
    ) {
        check_admin(&env, &env.invoker().into());

        let price = get_price(&env);
        let token = get_token(&env);

        let token_client = token::Client::new(&env, token.clone());
        let balance = token_client.balance(&get_contract_id(&env));
        let attendees: Vec<Identifier> = get_attendees(&env);
        let distribution_amount = balance.checked_div(attendees.len() as i128).unwrap();
        
        // The remainder will be left in the contract, and can be claimed in the future once
        // the balance increases.

        assert!(distribution_amount >= price);
        for attendee in attendees {
            transfer_from_contract_to_account(&env, &token, &attendee.unwrap(), &distribution_amount);
        }
    }
}

fn get_contract_id(e: &Env) -> Identifier {
    Identifier::Contract(e.get_current_contract())
}

fn transfer_from_account_to_contract(
    e: &Env,
    token_id: &BytesN<32>,
    from: &Identifier,
    amount: &i128,
) {
    let client = token::Client::new(e, token_id);
    client.xfer_from(&Signature::Invoker, &0, from, &get_contract_id(e), amount);
}

fn transfer_from_contract_to_account(
    e: &Env,
    token_id: &BytesN<32>,
    to: &Identifier,
    amount: &i128,
) {
    let client = token::Client::new(e, token_id);
    client.xfer(&Signature::Invoker, &0, to, amount);
}

mod test;
