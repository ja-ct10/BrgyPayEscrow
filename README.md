# BrgyPay

> Trustless escrow for Philippine barangay certificate fee payments — built on Stellar.

## Problem

Residents requesting certificates (clearance, residency, indigency) pay fees in cash
with no audit trail. Corrupt practices are difficult to detect, and disputes have no
on-chain ground truth. iBrgyPay solves this by holding fees in a Soroban escrow
contract that only releases funds when a certificate is officially issued.

## Solution

1. Barangay staff approve uploaded requirements in iBrgy → contract creates a request
2. Resident pays USDC into escrow via their Stellar wallet
3. Staff releases certificate → funds auto-settle to barangay treasury wallet
4. An on-chain receipt token is minted to the resident as tamper-proof proof of payment

## Timeline

| Stage | Description |
|---|---|
| Day 1–2 | Smart contract: `initialize`, `create_request`, `lock_payment` |
| Day 3 | `release_payment`, `refund`, and event emission |
| Day 4 | Testnet deployment + iBrgy backend integration (webhook on status change) |
| Day 5 | Frontend: resident pay button + treasury dashboard |
| Day 6 | Demo polish + video walkthrough |

## Stellar features used

- **USDC transfers** — stable peso-equivalent payments
- **Soroban smart contracts** — escrow hold and conditional release
- **Custom receipt token** — Stellar Asset Contract minted on release
- **Trustlines** — resident opts in to USDC before paying
- **Clawback** — admin can reclaim funds on fraud detection

## Target Users

42,000 barangays in the Philippines collect certificate fees daily.
iBrgyPay creates an auditable, corruption-resistant payment layer that any
LGU can plug into an existing system like iBrgy with one API endpoint.

## Prerequisites

- Rust (stable, 1.75+)
- Soroban CLI v21.x: `cargo install --locked soroban-cli`
- Stellar testnet account funded via Friendbot

## Build

```bash
soroban contract build
```

Output: `target/wasm32-unknown-unknown/release/brgypay_escrow.wasm`

## Test

```bash
cargo test
```

All 5 tests should pass with output confirming happy path, edge cases, and state.

## Deploy to testnet
`CAENNOHBSBPNM6ZZ7EKMSKUB4VZWED6L6RKUGPFXJFCAWDKOQWO5W2GU`

Explorer: 
https://stellar.expert/explorer/testnet/contract/CAENNOHBSBPNM6ZZ7EKMSKUB4VZWED6L6RKUGPFXJFCAWDKOQWO5W2GU

<img width="1919" height="1019" alt="image" src="https://github.com/user-attachments/assets/64d18129-a732-4785-b6b5-8e2e8d2e396b" />


## Initialize the contract

```bash
soroban contract invoke \
  --id CONTRACT_ID \
  --source YOUR_SECRET_KEY \
  --network testnet \
  -- initialize \
  --admin GBARANGAY_ADMIN_ADDRESS \
  --treasury GTREASURY_WALLET_ADDRESS
```

## Create a payment request (staff)

```bash
soroban contract invoke \
  --id CONTRACT_ID \
  --source ADMIN_SECRET_KEY \
  --network testnet \
  -- create_request \
  --resident GRESIDENT_WALLET_ADDRESS \
  --amount 1000000 \
  --token USDC_CONTRACT_ADDRESS
```

Returns: `request_id` (e.g. `1`)

## Lock payment (resident)

```bash
soroban contract invoke \
  --id CONTRACT_ID \
  --source RESIDENT_SECRET_KEY \
  --network testnet \
  -- lock_payment \
  --request_id 1
```

## Release (staff — triggers certificate issuance)

```bash
soroban contract invoke \
  --id CONTRACT_ID \
  --source ADMIN_SECRET_KEY \
  --network testnet \
  -- release_payment \
  --request_id 1
```

## License

MIT
