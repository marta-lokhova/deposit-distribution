#![no_std]

use soroban_sdk::{contractimpl, contracttype, BytesN, Env};

mod token {
    soroban_sdk::contractimport!(file = "soroban_token_spec.wasm");
}

use token::{Identifier, Signature};

#[derive(Clone)]
#[contracttype]
pub struct Attendee {
    pub fee: i128,
    pub attended: bool,
    pub refunded: bool
}

// TODO: add pricing tiers (can be set by admin)
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Attendee,
    Count,
    Unclaimed,
    Price,
    Token
}

pub struct DistributionContract;

fn get_price(e: &Env) -> i128 {
    e.storage().get_unchecked(DataKey::Price).unwrap()
}

fn get_token(e: &Env) -> BytesN<32> {
    e.storage().get_unchecked(DataKey::Token).unwrap()
}

fn get_count(e: &Env) -> u32 {
    e.storage().get_unchecked(DataKey::Count).unwrap()
}

fn get_unclaimed(e: &Env) -> i128 {
    e.storage().get_unchecked(DataKey::Unclaimed).unwrap()
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

        e.storage().set(DataKey::Price, price);
        e.storage().set(DataKey::Token, token);
        e.storage().set(DataKey::Unclaimed, 0 as i128);
        e.storage().set(DataKey::Count, 0 as u32);
    }

    pub fn deposit(
        env: Env,
        attendee: Identifier
    ) {
        if attendee == read_administrator(&env)
        {
            panic!("admin cannot deposit")
        }

        let price = get_price(&env);
        let token = get_token(&env);

        if env.storage().has(attendee.clone()) {
            panic!("attendee already registered");
        }

        let attendee_struct = Attendee{fee: price, attended: false, refunded: false};
        env.storage().set(&attendee, attendee_struct);

        let mut unclaimed: i128 = get_unclaimed(&env);
        unclaimed += price;
        env.storage().set(DataKey::Unclaimed, unclaimed);

        // Transfer token to this contract address.
        transfer_from_account_to_contract(&env, &token, &attendee.into(), &price);
    }
    
    pub fn attend(
        env: Env,
        attendee: Identifier
    ) {
        check_admin(&env, &env.invoker().into());
        if attendee == read_administrator(&env)
        {
            panic!("admin cannot attend")
        }

        if !env.storage().has(attendee.clone()) {
            panic!("attendee did not register");
        }

        let mut stored_att : Attendee = env.storage().get_unchecked(attendee.clone()).unwrap();

        if stored_att.attended
        {
            panic!("attendance already recorded")
        } 

        stored_att.attended = true;
        env.storage().set(&attendee, stored_att);

        // Store withdrawal ID
        let mut count: u32 = get_count(&env);
        env.storage().set(count, attendee);

        // Increment and save the count.
        count += 1;
        env.storage().set(DataKey::Count, &count);

        // Decrement unclaimed 
        let mut unclaimed: i128 = get_unclaimed(&env);
        let price = get_price(&env);

        // Decrement and save unclaimed
        unclaimed -= price;
        env.storage().set(DataKey::Unclaimed, unclaimed);

    }

    // Distribute the money to a batch of attendees
    pub fn withdraw(
        env: Env,
        high: u32,
        low: u32,
    ) -> i32 {
        // TODO; once withdrawal started, deposit and attend should not be allowed
        check_admin(&env, &env.invoker().into());

        if high < low || high - low > 10
        {
            panic!("Invalid range")
        }

        let price = get_price(&env);
        let token = get_token(&env);
        let withdrawal_count = get_count(&env);
        let unclaimed = get_unclaimed(&env);

        let distribution_amount = price + unclaimed.checked_div(withdrawal_count as i128).unwrap();
        
        // The remainder will be left in the contract, and can be claimed in the future once
        // the balance increases.
        let mut refund_count = 0;
        for id in low..high {
            if !env.storage().has(id)
            {
                continue;
            }

            let att : Identifier = env.storage().get_unchecked(id).unwrap();
            let mut att_struct : Attendee = env.storage().get_unchecked(&att).unwrap();

            if !att_struct.refunded
            {
                transfer_from_contract_to_account(&env, &token, &att, &distribution_amount);
                att_struct.refunded = true;
                env.storage().set(att, att_struct);
                refund_count += 1
            }
        }
        refund_count
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
