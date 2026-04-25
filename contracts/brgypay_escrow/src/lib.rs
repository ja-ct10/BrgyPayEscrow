#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    token, Address, Env, Symbol,
};

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Request(u64),   // request_id → RequestRecord
    Treasury,       // barangay treasury wallet
    Admin,          // contract admin (barangay system)
    RequestCount,   // monotonic counter
}

// ── Data types ────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum RequestStatus {
    Pending,    // staff approved docs, awaiting payment
    Paid,       // funds locked in escrow
    Released,   // certificate issued, funds settled
    Refunded,   // rejected after payment
}

#[contracttype]
#[derive(Clone)]
pub struct RequestRecord {
    pub request_id: u64,
    pub resident:   Address,    // resident's Stellar wallet
    pub amount:     i128,       // fee in USDC stroops (1 USDC = 10_000_000)
    pub token:      Address,    // USDC token contract address
    pub status:     RequestStatus,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct IBrgyPay;

#[contractimpl]
impl IBrgyPay {

    /// Initialize the contract. Called once on deployment.
    /// Sets the admin (barangay system) and treasury wallet.
    pub fn initialize(env: Env, admin: Address, treasury: Address) {
        // Prevent re-initialization
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.storage().instance().set(&DataKey::RequestCount, &0u64);
    }

    /// Called by barangay staff after verifying the resident's documents.
    /// Registers a payment request and returns the request_id.
    /// Only the admin (barangay system backend) may call this.
    pub fn create_request(
        env:       Env,
        resident:  Address,
        amount:    i128,
        token:     Address,
    ) -> u64 {
        // Verify caller is the registered admin
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Increment request counter
        let mut count: u64 = env
            .storage().instance()
            .get(&DataKey::RequestCount)
            .unwrap_or(0);
        count += 1;

        let record = RequestRecord {
            request_id: count,
            resident,
            amount,
            token,
            status: RequestStatus::Pending,
        };

        env.storage().instance().set(&DataKey::Request(count), &record);
        env.storage().instance().set(&DataKey::RequestCount, &count);

        // Emit event so the iBrgy frontend can listen
        env.events().publish(
            (symbol_short!("created"), count),
            record.resident.clone(),
        );

        count
    }

    /// Called by the resident to pay the fee.
    /// Transfers USDC from the resident's wallet into this contract (escrow).
    pub fn lock_payment(env: Env, request_id: u64) {
        let mut record: RequestRecord = env
            .storage().instance()
            .get(&DataKey::Request(request_id))
            .expect("request not found");

        // Only the correct resident may pay
        record.resident.require_auth();

        // Request must be in Pending state
        if record.status != RequestStatus::Pending {
            panic!("invalid status");
        }

        // Transfer USDC from resident → this contract
        let token_client = token::Client::new(&env, &record.token);
        token_client.transfer(
            &record.resident,
            &env.current_contract_address(),
            &record.amount,
        );

        // Update status
        record.status = RequestStatus::Paid;
        env.storage().instance().set(&DataKey::Request(request_id), &record);

        env.events().publish(
            (symbol_short!("paid"), request_id),
            record.amount,
        );
    }

    /// Called by barangay staff to release the certificate.
    /// Settles escrowed USDC to the barangay treasury and marks request Released.
    pub fn release_payment(env: Env, request_id: u64) {
        // Only admin may release
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut record: RequestRecord = env
            .storage().instance()
            .get(&DataKey::Request(request_id))
            .expect("request not found");

        if record.status != RequestStatus::Paid {
            panic!("payment not locked");
        }

        let treasury: Address = env.storage().instance().get(&DataKey::Treasury).unwrap();

        // Transfer USDC from this contract → treasury
        let token_client = token::Client::new(&env, &record.token);
        token_client.transfer(
            &env.current_contract_address(),
            &treasury,
            &record.amount,
        );

        record.status = RequestStatus::Released;
        env.storage().instance().set(&DataKey::Request(request_id), &record);

        // The receipt token mint is handled off-chain (SAC custom asset)
        // but the on-chain status is the ground truth for the frontend
        env.events().publish(
            (symbol_short!("released"), request_id),
            treasury,
        );
    }

    /// Called by admin to refund a resident if a request is rejected post-payment.
    /// Clawback path — returns escrowed USDC to the resident.
    pub fn refund(env: Env, request_id: u64) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut record: RequestRecord = env
            .storage().instance()
            .get(&DataKey::Request(request_id))
            .expect("request not found");

        if record.status != RequestStatus::Paid {
            panic!("nothing to refund");
        }

        let token_client = token::Client::new(&env, &record.token);
        token_client.transfer(
            &env.current_contract_address(),
            &record.resident,
            &record.amount,
        );

        record.status = RequestStatus::Refunded;
        env.storage().instance().set(&DataKey::Request(request_id), &record);

        env.events().publish(
            (symbol_short!("refunded"), request_id),
            record.resident.clone(),
        );
    }

    /// Read a request's current status. Used by the iBrgy frontend.
    pub fn get_request(env: Env, request_id: u64) -> RequestRecord {
        env.storage().instance()
            .get(&DataKey::Request(request_id))
            .expect("request not found")
    }
}

#[cfg(test)]
mod test;