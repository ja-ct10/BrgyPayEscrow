#[cfg(test)]
mod tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        token, Address, Env, IntoVal,
    };

    use crate::{BrgyPayEscrow, BrgyPayEscrowClient};

    // -----------------------------------------------------------------------
    // Helper: deploys a mock USDC token and mints to an address
    // -----------------------------------------------------------------------
    fn create_token<'a>(env: &Env, admin: &Address) -> (Address, token::Client<'a>) {
        let token_id = env.register_stellar_asset_contract(admin.clone());
        let token_client = token::Client::new(env, &token_id);
        (token_id, token_client)
    }

    fn setup() -> (Env, Address, Address, Address, i128, u64) {
        let env = Env::default();
        env.mock_all_auths();

        let requester = Address::generate(&env);
        let courier = Address::generate(&env);
        let admin = Address::generate(&env);

        let (token_id, token_client) = create_token(&env, &admin);

        let amount: i128 = 3_000_000; // 3 USDC (6 decimals)
        // Mint USDC to requester so they can fund the escrow
        token_client.mint(&requester, &amount);

        let deadline: u64 = env.ledger().sequence() + 100;

        (env, requester, courier, token_id, amount, deadline)
    }

    // -----------------------------------------------------------------------
    // Test 1 — Happy path: requester locks funds, confirms delivery,
    // courier receives USDC
    // -----------------------------------------------------------------------
    #[test]
    fn test_happy_path_confirm_delivery() {
        let (env, requester, courier, token_id, amount, deadline) = setup();
        let contract_id = env.register_contract(None, BrgyPayEscrow);
        let client = BrgyPayEscrowClient::new(&env, &contract_id);

        client.initialize(&requester, &courier, &token_id, &amount, &deadline);
        client.confirm_delivery();

        // Courier should now hold the full escrowed amount
        let token_client = token::Client::new(&env, &token_id);
        assert_eq!(token_client.balance(&courier), amount);
        assert_eq!(token_client.balance(&contract_id), 0);
    }

    // -----------------------------------------------------------------------
    // Test 2 — Edge case: calling confirm_delivery twice must panic
    // -----------------------------------------------------------------------
    #[test]
    #[should_panic(expected = "escrow already settled")]
    fn test_double_confirm_panics() {
        let (env, requester, courier, token_id, amount, deadline) = setup();
        let contract_id = env.register_contract(None, BrgyPayEscrow);
        let client = BrgyPayEscrowClient::new(&env, &contract_id);

        client.initialize(&requester, &courier, &token_id, &amount, &deadline);
        client.confirm_delivery();
        // Second call must panic — funds are already released
        client.confirm_delivery();
    }

    // -----------------------------------------------------------------------
    // Test 3 — State verification: after initialize, storage reflects
    // correct confirmed=false, disputed=false, and locked amount
    // -----------------------------------------------------------------------
    #[test]
    fn test_state_after_initialize() {
        let (env, requester, courier, token_id, amount, deadline) = setup();
        let contract_id = env.register_contract(None, BrgyPayEscrow);
        let client = BrgyPayEscrowClient::new(&env, &contract_id);

        client.initialize(&requester, &courier, &token_id, &amount, &deadline);

        let (confirmed, disputed, stored_amount, stored_deadline) = client.get_status();
        assert!(!confirmed);
        assert!(!disputed);
        assert_eq!(stored_amount, amount);
        assert_eq!(stored_deadline, deadline);

        // Contract address holds the locked funds
        let token_client = token::Client::new(&env, &token_id);
        assert_eq!(token_client.balance(&contract_id), amount);
    }

    // -----------------------------------------------------------------------
    // Test 4 — Dispute: requester disputes and gets full refund
    // -----------------------------------------------------------------------
    #[test]
    fn test_dispute_refunds_requester() {
        let (env, requester, courier, token_id, amount, deadline) = setup();
        let contract_id = env.register_contract(None, BrgyPayEscrow);
        let client = BrgyPayEscrowClient::new(&env, &contract_id);

        client.initialize(&requester, &courier, &token_id, &amount, &deadline);
        client.dispute();

        let token_client = token::Client::new(&env, &token_id);
        // Requester gets all funds back
        assert_eq!(token_client.balance(&requester), amount);
        assert_eq!(token_client.balance(&courier), 0);
        assert_eq!(token_client.balance(&contract_id), 0);
    }

    // -----------------------------------------------------------------------
    // Test 5 — Timeout claim: courier claims after deadline passes
    // -----------------------------------------------------------------------
    #[test]
    fn test_courier_claims_after_timeout() {
        let (env, requester, courier, token_id, amount, deadline) = setup();
        let contract_id = env.register_contract(None, BrgyPayEscrow);
        let client = BrgyPayEscrowClient::new(&env, &contract_id);

        client.initialize(&requester, &courier, &token_id, &amount, &deadline);

        // Advance ledger sequence past the deadline
        env.ledger().set(LedgerInfo {
            sequence_number: deadline + 1,
            timestamp: 12345,
            protocol_version: 20,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        client.claim_timeout();

        let token_client = token::Client::new(&env, &token_id);
        assert_eq!(token_client.balance(&courier), amount);
        assert_eq!(token_client.balance(&contract_id), 0);
    }
}