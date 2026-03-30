# English Auction Contract for Minichain

This package contains the canonical auction contract source, design notes, and Bun E2E coverage for the English auction example:

- contract source in `src/auction.asm`
- runtime notes in `AUCTION_DESIGN.md`
- chain-level tests under `test/`

## What This Contract Does

The contract implements an ascending-price auction with:

- seller initialization on first execution
- reserve price `100`
- minimum bid increment `10`
- auction duration `3600` seconds from deployment
- pending refund slots for outbid bidders

Dispatch is intentionally simple:

- `CALLVALUE > 0` means `bid()`
- `CALLVALUE == 0` routes to `withdraw()` or `seller_withdraw()` based on the caller

## Quick Start

### 1. Initialize a chain and create accounts

```bash
cargo run --release -- init --authorities 1

cargo run --release -- account new --name alice
cargo run --release -- account new --name bob
cargo run --release -- account new --name charlie
```

### 2. Fund bidder accounts with native MIC

```bash
cargo run --release -- account mint --from @authority_0 --to <ALICE_ADDR> --amount 1000000
cargo run --release -- account mint --from @authority_0 --to <BOB_ADDR> --amount 500000
cargo run --release -- account mint --from @authority_0 --to <CHARLIE_ADDR> --amount 500000
```

### 3. Deploy the auction contract

```bash
cargo run --release -- deploy \
  --from @alice \
  --source contracts/auction/src/auction.asm \
  --gas-limit 500000
```

Then produce a block:

```bash
cargo run --release -- block produce --authority authority_0
```

The deploy command prints:

```text
Contract Address: 0x...
```

### 4. Initialize the contract state

The auction stores its initial seller/reserve/end-time values on the first call, not during deployment. Have the seller send one zero-value call after deployment:

```bash
cargo run --release -- call --from @alice --to 0x... --amount 0 --gas-limit 250000
cargo run --release -- block produce --authority authority_0
```

## CLI Usage

### Place bids

```bash
# Bob bids 150
cargo run --release -- call \
  --from @bob \
  --to 0x... \
  --amount 150 \
  --gas-limit 250000

cargo run --release -- block produce --authority authority_0
```

```bash
# Charlie outbids with 200
cargo run --release -- call \
  --from @charlie \
  --to 0x... \
  --amount 200 \
  --gas-limit 250000

cargo run --release -- block produce --authority authority_0
```

### Withdraw and seller claim

```bash
# Outbid bidder attempts refund withdrawal
cargo run --release -- call --from @bob --to 0x... --amount 0 --gas-limit 250000

# Seller attempts proceeds withdrawal after the auction duration has elapsed
cargo run --release -- call --from @alice --to 0x... --amount 0 --gas-limit 250000
```

## Current Runtime Limitations

The auction contract writes internal state correctly, but the current chain runtime does not yet convert the contract's `LOG` outputs into native MIC payouts. In practice:

- successful bids move native MIC into the contract balance
- refund and seller-withdraw calls do not yet pay native MIC back out
- the one-hour auction duration makes end-of-auction chain E2E expensive to test in real time

The Bun suite in this package therefore focuses on deploy and active bidding flows, while the Rust VM tests still cover the full contract logic.

## Testing

Build the CLI and run the package tests:

```bash
cargo build --release -p minichain-cli
bun test contracts/auction
```

The chain-level suite covers:

- deploy from the package source path
- first funded call after deployment
- additional bids increasing contract balance
- higher outbid funding the contract further
- current no-payout behavior for refund withdrawal
