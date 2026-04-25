#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, Symbol,
};

// ---------------------------------------------------------------------------
// Storage key enum — each variant is a distinct key in contract storage
// ---------------------------------------------------------------------------
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Requester,       // Address that locked the funds
    Courier,         // Address that will receive funds on delivery
    Amount,          // USDC amount locked (in stroops-equivalent token units)
    Token,           // USDC token contract address
    Confirmed,       // bool — has the requester confirmed delivery?
    Disputed,        // bool — has the requester raised a dispute?
    Deadline,        // u64 ledger timestamp — auto-release deadline
}

// ---------------------------------------------------------------------------
// Contract struct
// ---------------------------------------------------------------------------
#[contract]
pub struct BrgyPayEscrow;

#[contractimpl]
impl BrgyPayEscrow {

    // -----------------------------------------------------------------------
    // initialize — called once by the requester to lock USDC into escrow
    // requester: the store owner's Stellar address
    // courier: the delivery person's Stellar address
    // token: the USDC token contract address on Stellar
    // amount: how many token units to lock (e.g. 3_000_000 = 3 USDC at 6 decimals)
    // deadline_ledger: ledger number after which courier can auto-claim
    // -----------------------------------------------------------------------
    pub fn initialize(
        env: Env,
        requester: Address,
        courier: Address,
        token: Address,
        amount: i128,
        deadline_ledger: u64,
    ) {
        // Prevent re-initialization — contract can only be set up once
        if env.storage().instance().has(&DataKey::Requester) {
            panic!("already initialized");
        }

        // Requester must authorize this call — they are locking their own funds
        requester.require_auth();

        // Transfer USDC from requester into this contract's address
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&requester, &env.current_contract_address(), &amount);

        // Persist all escrow parameters to contract storage
        env.storage().instance().set(&DataKey::Requester, &requester);
        env.storage().instance().set(&DataKey::Courier, &courier);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Amount, &amount);
        env.storage().instance().set(&DataKey::Confirmed, &false);
        env.storage().instance().set(&DataKey::Disputed, &false);
        env.storage().instance().set(&DataKey::Deadline, &deadline_ledger);

        // Emit an event so off-chain indexers can track escrow creation
        env.events().publish(
            (Symbol::new(&env, "escrow_created"),),
            (requester, courier, amount),
        );
    }

    // -----------------------------------------------------------------------
    // confirm_delivery — called by the requester after receiving the certificate
    // Releases escrowed USDC to the courier immediately
    // -----------------------------------------------------------------------
    pub fn confirm_delivery(env: Env) {
        // Only the requester can confirm delivery
        let requester: Address = env.storage().instance().get(&DataKey::Requester).unwrap();
        requester.require_auth();

        // Guard: cannot confirm if already confirmed or disputed
        let confirmed: bool = env.storage().instance().get(&DataKey::Confirmed).unwrap();
        let disputed: bool = env.storage().instance().get(&DataKey::Disputed).unwrap();
        if confirmed || disputed {
            panic!("escrow already settled");
        }

        // Mark as confirmed before transferring (checks-effects-interactions)
        env.storage().instance().set(&DataKey::Confirmed, &true);

        // Release funds to the courier
        let courier: Address = env.storage().instance().get(&DataKey::Courier).unwrap();
        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let amount: i128 = env.storage().instance().get(&DataKey::Amount).unwrap();

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &courier, &amount);

        env.events().publish(
            (Symbol::new(&env, "delivery_confirmed"),),
            (courier, amount),
        );
    }

    // -----------------------------------------------------------------------
    // dispute — called by the requester if the courier fails to deliver
    // Returns escrowed USDC to the requester
    // -----------------------------------------------------------------------
    pub fn dispute(env: Env) {
        let requester: Address = env.storage().instance().get(&DataKey::Requester).unwrap();
        requester.require_auth();

        let confirmed: bool = env.storage().instance().get(&DataKey::Confirmed).unwrap();
        let disputed: bool = env.storage().instance().get(&DataKey::Disputed).unwrap();
        if confirmed || disputed {
            panic!("escrow already settled");
        }

        // Mark disputed and refund requester
        env.storage().instance().set(&DataKey::Disputed, &true);

        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let amount: i128 = env.storage().instance().get(&DataKey::Amount).unwrap();

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &requester, &amount);

        env.events().publish(
            (Symbol::new(&env, "dispute_resolved"),),
            (requester, amount),
        );
    }

    // -----------------------------------------------------------------------
    // claim_timeout — called by the courier after the deadline passes
    // Auto-releases funds if requester never confirmed or disputed
    // -----------------------------------------------------------------------
    pub fn claim_timeout(env: Env) {
        let courier: Address = env.storage().instance().get(&DataKey::Courier).unwrap();
        courier.require_auth();

        let confirmed: bool = env.storage().instance().get(&DataKey::Confirmed).unwrap();
        let disputed: bool = env.storage().instance().get(&DataKey::Disputed).unwrap();
        if confirmed || disputed {
            panic!("escrow already settled");
        }

        // Ensure the deadline ledger has passed
        let deadline: u64 = env.storage().instance().get(&DataKey::Deadline).unwrap();
        if env.ledger().sequence() < deadline {
            panic!("deadline not yet reached");
        }

        env.storage().instance().set(&DataKey::Confirmed, &true);

        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let amount: i128 = env.storage().instance().get(&DataKey::Amount).unwrap();

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &courier, &amount);

        env.events().publish(
            (Symbol::new(&env, "timeout_claimed"),),
            (courier, amount),
        );
    }

    // -----------------------------------------------------------------------
    // get_status — read-only view of escrow state
    // Returns (confirmed, disputed, amount, deadline)
    // -----------------------------------------------------------------------
    pub fn get_status(env: Env) -> (bool, bool, i128, u64) {
        let confirmed: bool = env.storage().instance().get(&DataKey::Confirmed).unwrap();
        let disputed: bool = env.storage().instance().get(&DataKey::Disputed).unwrap();
        let amount: i128 = env.storage().instance().get(&DataKey::Amount).unwrap();
        let deadline: u64 = env.storage().instance().get(&DataKey::Deadline).unwrap();
        (confirmed, disputed, amount, deadline)
    }
}