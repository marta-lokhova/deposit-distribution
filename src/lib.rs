//! This contract demonstrates 'timelock' concept and implements a
//! greatly simplified Claimable Balance (similar to
//! https://developers.stellar.org/docs/glossary/claimable-balance).
//! The contract allows to deposit some amount of token and allow another
//! account(s) claim it before or after provided time point.
//! For simplicity, the contract only supports invoker-based auth.
#![no_std]

use soroban_sdk::{contractimpl, contracttype, BytesN, Env, Vec};

mod token {
    soroban_sdk::contractimport!(file = "soroban_token_spec.wasm");
}

use token::{Identifier, Signature};

// TODO: audit get, get_unchecked, need to ensure errors are handled gracefully
// TODO: add admin
// TODO: add pricing tiers (can be set by admin)
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Attendees,
    Price
}

pub struct DistributionContract;

fn get_attendees(e: &Env) -> Vec<Identifier> {
    e.storage().get_unchecked(DataKey::Attendees).unwrap()
}

fn get_price(e: &Env) -> i128 {
    e.storage().get_unchecked(DataKey::Price).unwrap()
}

#[contractimpl]
impl DistributionContract {

    pub fn initialize(
        e: Env,
        price: i128,
    ) {
        e.storage().set(DataKey::Price, price);
    }

    pub fn deposit(
        env: Env,
        token: BytesN<32>,
        attendee: Identifier
    ) {
        let price = get_price(&env);
        // Transfer token to this contract address.
        transfer_from_account_to_contract(&env, &token, &attendee.into(), &price);
        // TODO: what to do if transfer fails
        // TODO: maybe store the list of depositors if needed later
    }
    
    pub fn attend(
        env: Env,
        attendee: Identifier
    ) {
        // Store actual attendees on chain
        let mut attendees: Vec<Identifier> = get_attendees(&env);
        attendees.push_back(attendee);
        env.storage().set(
            DataKey::Attendees,
            attendees
        )
    }

    // Distribute the money to everyone
    pub fn withdraw(
        env: Env,
        token_id: BytesN<32>
    ) {
        let price = get_price(&env);

        let attendees: Vec<Identifier> = get_attendees(&env);
        for attendee in attendees {
            transfer_from_contract_to_account(&env, &token_id, &attendee.unwrap(), &price);
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
