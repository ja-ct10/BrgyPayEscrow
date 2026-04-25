# BrgyPay Escrow

Trustless USDC escrow for barangay certificate delivery in the Philippines.

## Problem
Sari-sari store owners in Metro Manila pay couriers upfront to deliver barangay
business certificates, losing ₱300–₱500 with no recourse when couriers disappear.

## Solution
A Soroban smart contract locks USDC until the requester confirms receipt. Stellar's
5-second finality and sub-cent fees make micro-escrow viable for ₱100–₱500 transactions.

## Stellar Features Used
- USDC (Circle Stellar-native) transfers
- Soroban smart contracts (escrow, timeout, dispute)
- Trustlines (courier wallet must accept USDC)

## Prerequisites
- Rust 1.74+
- Soroban CLI 20.x: `cargo install --locked soroban-cli`
- Stellar testnet account funded via [Friendbot](https://friendbot.stellar.org)

## Build
```bash
soroban contract build
```

## Test
```bash
cargo test
```

## Deploy to Testnet
```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/brgypay_escrow.wasm \
  --source <YOUR_SECRET_KEY> \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"
```

## Sample CLI Invocation — Initialize Escrow
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <REQUESTER_SECRET_KEY> \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- initialize \
  --requester GABC...1234 \
  --courier GXYZ...5678 \
  --token GUSDC...TOKEN_ADDRESS \
  --amount 3000000 \
  --deadline_ledger 1500000
```

## Sample CLI Invocation — Confirm Delivery
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <REQUESTER_SECRET_KEY> \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- confirm_delivery
```

## Vision
BrgyPay Escrow bridges the iBrgy government certificate system with trustless
on-chain payments — making every barangay transaction safe for the 50,000+
barangays across the Philippines without requiring either party to trust the other.

## License
MIT
