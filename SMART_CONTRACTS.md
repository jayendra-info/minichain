# Smart Contract Ideas for Minichain

This document contains non-trivial smart contract suggestions for demonstrating minichain's capabilities. These contracts go beyond simple counters and showcase real-world blockchain patterns.

## Table of Contents

1. [ERC-20 Token Contract](#1-erc-20-token-contract-)
2. [Multi-Signature Wallet](#2-multi-signature-wallet-)
3. [Escrow Contract](#3-escrow-contract-) ⭐ **Recommended Starting Point**
4. [English Auction](#4-english-auction-)
5. [Simple DAO Voting](#5-simple-dao-voting-)

---

## 1. ERC-20 Token Contract 📊

**Complexity:** Medium | **Estimated Lines:** ~150-200 assembly

### Features

- Total supply tracking
- Balance mapping per address
- Transfer tokens between accounts
- Approve/transferFrom allowance pattern
- Events simulation via storage

### Why Interesting

- Industry standard, everyone knows it
- Teaches mapping patterns (address → balance)
- Demonstrates allowance delegation
- Good for testing with multiple accounts
- Foundation for DeFi applications

### Storage Layout

```
slot 0: total_supply
slot 1: owner_address
slot hash(address, 0): balances[address]
slot hash(address, hash(spender, 1)): allowances[owner][spender]
```

### Key Functions

- `totalSupply()` → Returns total token supply
- `balanceOf(address)` → Returns balance of address
- `transfer(to, amount)` → Transfer tokens to address
- `approve(spender, amount)` → Allow spender to use tokens
- `transferFrom(from, to, amount)` → Transfer on behalf of owner
- `allowance(owner, spender)` → Check approved amount

### CLI Usage Example

```bash
# Deploy token contract
minichain deploy --from alice --source erc20.asm

# Transfer 100 tokens from Alice to Bob
minichain call --from alice --to TOKEN_ADDR --data <transfer_calldata>

# Check Bob's balance
minichain call --from bob --to TOKEN_ADDR --data <balanceOf_calldata>

# Alice approves Bob to spend 50 tokens
minichain call --from alice --to TOKEN_ADDR --data <approve_calldata>

# Bob transfers 30 tokens from Alice to Charlie (using allowance)
minichain call --from bob --to TOKEN_ADDR --data <transferFrom_calldata>
```

### Implementation Considerations

- Hash-based storage mapping: `hash(address || slot_id)` for balances
- Prevent overflow/underflow in arithmetic
- Check sender has sufficient balance before transfer
- Check allowance before transferFrom
- Update allowances after transferFrom

---

## 2. Multi-Signature Wallet 🔐

**Complexity:** Medium-High | **Estimated Lines:** ~200-250 assembly

### Features

- Multiple owners (2-of-3 or 3-of-5)
- Submit transaction proposals
- Sign/approve proposals
- Execute when threshold met
- Revoke signature before execution
- Proposal expiration

### Why Interesting

- Real security pattern used in production (Gnosis Safe)
- Demonstrates access control
- Shows signature verification patterns
- State machine with proposal lifecycle
- Critical for treasury management

### Storage Layout

```
slot 0: owner_count
slot 1: required_signatures
slot 2: next_proposal_id
slot hash(idx, 0): owners[idx]
slot hash(proposal_id, 2): proposal_destination
slot hash(proposal_id, 3): proposal_value
slot hash(proposal_id, 4): proposal_data_hash
slot hash(proposal_id, 5): signature_count
slot hash(proposal_id, 6): executed
slot hash(proposal_id, hash(signer, 7)): has_signed
```

### Key Functions

- `isOwner(address)` → Check if address is an owner
- `submitProposal(to, value, data)` → Create new proposal
- `signProposal(proposalId)` → Sign a proposal
- `revokeSignature(proposalId)` → Revoke signature
- `executeProposal(proposalId)` → Execute if threshold met
- `getSignatureCount(proposalId)` → Check signatures

### CLI Usage Example

```bash
# Deploy 2-of-3 multisig (Alice, Bob, Charlie)
minichain deploy --from alice --source multisig.asm

# Alice submits proposal to send 100 tokens
minichain call --from alice --to MULTISIG --data <submit_proposal>

# Bob signs the proposal
minichain call --from bob --to MULTISIG --data <sign_proposal>

# 2 signatures reached, anyone can execute
minichain call --from charlie --to MULTISIG --data <execute_proposal>

# Funds sent!
```

### Use Cases

- Treasury management requiring multiple approvals
- Shared company wallet
- Joint account for partners
- High-value transaction authorization
- Emergency shutdown mechanisms

### Implementation Considerations

- Only owners can submit/sign proposals
- Cannot sign twice
- Cannot execute before threshold
- Cannot execute after expiration
- Prevent replay attacks with proposal IDs

---

## 3. Escrow Contract 🤝

**Complexity:** Medium | **Estimated Lines:** ~120-150 assembly

### ⭐ **Recommended as First Non-Trivial Contract**

### Features

- Buyer deposits funds
- Seller delivers (off-chain verification)
- Arbiter can release to seller or refund buyer
- Timeout mechanism (release after N blocks)
- Dispute resolution
- Automated refunds on timeout

### Why Interesting

- Classic blockchain use case
- State machine: `Created → Funded → Released/Refunded`
- Time-locked logic (timestamp checks)
- Three-party interaction (buyer, seller, arbiter)
- Clear success criteria (funds move correctly)
- Practical and easy to understand

### Storage Layout

```
slot 0: buyer_address
slot 1: seller_address
slot 2: arbiter_address
slot 3: amount
slot 4: deadline_timestamp
slot 5: state (0=created, 1=funded, 2=released, 3=refunded, 4=disputed)
```

### State Machine

```
Created (0)
    ↓ (buyer deposits)
Funded (1)
    ↓ (arbiter releases OR timeout)
Released (2) → Seller withdraws
    OR
    ↓ (arbiter refunds OR buyer disputes)
Refunded (3) → Buyer withdraws
```

### Key Functions

- `deposit()` → Buyer funds the escrow
- `release()` → Arbiter releases to seller
- `refund()` → Arbiter refunds to buyer
- `dispute()` → Buyer raises dispute
- `claimTimeout()` → Auto-release after deadline
- `withdraw()` → Winner withdraws funds

### CLI Usage Example

```bash
# 1. Deploy escrow (buyer=alice, seller=bob, arbiter=charlie)
# Deadline: 7 days from now
minichain deploy --from alice --source escrow.asm

# 2. Alice deposits 1000 tokens
minichain call --from alice --to ESCROW --amount 1000
minichain block produce --authority authority_0

# 3. Bob delivers goods (off-chain)

# 4a. Happy path: Charlie releases to Bob
minichain call --from charlie --to ESCROW --data <release>
minichain block produce --authority authority_0

# 4b. OR Dispute: Alice claims non-delivery
minichain call --from alice --to ESCROW --data <dispute>
# Charlie investigates and decides
minichain call --from charlie --to ESCROW --data <refund>

# 5. Winner withdraws
minichain call --from bob --to ESCROW --data <withdraw>
```

### Use Cases

- E-commerce with blockchain payments
- Freelance work payment protection
- Real estate transactions
- Equipment rentals
- Peer-to-peer marketplace

### Implementation Considerations

- Only buyer can deposit
- Only arbiter can release/refund
- Deadline must be in future
- Cannot change state after finalized
- Prevent re-entrancy on withdraw
- Check timestamp for timeout

### Why This Is The Best Starting Point

1. **Clear Success Criteria** - Easy to verify: funds move from buyer → escrow → seller
2. **Multiple Accounts** - Demonstrates three-party interaction
3. **State Machine** - Shows proper state transitions
4. **Practical Use Case** - Everyone understands escrow
5. **Right Complexity** - Not too simple (counter), not too complex (DAO)
6. **Time Logic** - Demonstrates timestamp/block-based conditions
7. **Good Testing** - Can test happy path, disputes, timeouts
8. **Real-World Pattern** - Actually used in production systems

---

## 4. English Auction 🔨

**Complexity:** Medium | **Estimated Lines:** ~150-180 assembly

### Features

- Seller lists item with reserve price
- Bidders place increasing bids
- Previous bidder gets refund automatically
- Auction ends after timestamp
- Winner claims, seller withdraws funds
- Minimum bid increment

### Why Interesting

- Time-based logic (block timestamp)
- Automatic refunds (pull over push pattern)
- Highest bidder tracking
- Finality conditions
- Economic game theory

### Storage Layout

```
slot 0: seller_address
slot 1: highest_bid
slot 2: highest_bidder
slot 3: auction_end_time
slot 4: reserve_price
slot 5: ended (bool)
slot 6: minimum_bid_increment
slot hash(bidder, 0): pending_returns[bidder]
```

### State Flow

```
Created (seller deploys)
    ↓
Active (bidding open, before end_time)
    ↓
Ended (after end_time, winner determined)
    ↓
Finalized (funds withdrawn)
```

### Key Functions

- `bid()` → Place a bid (must be higher)
- `withdraw()` → Outbid bidders withdraw
- `endAuction()` → Finalize after end_time
- `sellerWithdraw()` → Seller claims highest bid
- `getHighestBid()` → Query current highest
- `timeRemaining()` → Check time left

### CLI Usage Example

```bash
# Alice auctions an NFT (conceptual), 1 hour duration
minichain deploy --from alice --source auction.asm

# Bob bids 100
minichain call --from bob --to AUCTION --amount 100
minichain block produce --authority authority_0

# Charlie bids 150 (Bob gets 100 back automatically)
minichain call --from charlie --to AUCTION --amount 150
minichain block produce --authority authority_0

# Dave bids 200 (Charlie gets 150 back)
minichain call --from dave --to AUCTION --amount 200
minichain block produce --authority authority_0

# Wait for auction end...
# (time passes beyond end_time)

# Anyone can call end auction
minichain call --from anyone --to AUCTION --data <end_auction>
minichain block produce --authority authority_0

# Alice (seller) withdraws 200
minichain call --from alice --to AUCTION --data <seller_withdraw>
```

### Use Cases

- NFT auctions
- Domain name sales
- Art sales
- Liquidation auctions
- Asset disposal

### Implementation Considerations

- Bid must exceed highest bid by minimum increment
- Refund previous bidder immediately (push pattern)
- Cannot bid after auction ends
- Reserve price must be met
- Prevent seller from bidding
- Handle case where no valid bids (refund to seller)

---

## 5. Simple DAO Voting 🗳️

**Complexity:** Medium-High | **Estimated Lines:** ~180-220 assembly

### Features

- Members hold voting tokens
- Create proposals (text hash + action)
- Vote yes/no with token weight
- Quorum requirement
- Execute proposal if passed
- Proposal expiration
- Vote delegation

### Why Interesting

- Governance primitive
- Weighted voting by balance
- Demonstrates on-chain decision making
- Can trigger other contract calls
- Combines multiple patterns

### Storage Layout

```
slot 0: next_proposal_id
slot 1: total_voting_power
slot 2: quorum_percentage (e.g., 50 = 50%)
slot hash(proposal_id, 0): proposal_description_hash
slot hash(proposal_id, 1): proposal_target_contract
slot hash(proposal_id, 2): proposal_calldata_hash
slot hash(proposal_id, 3): yes_votes
slot hash(proposal_id, 4): no_votes
slot hash(proposal_id, 5): deadline
slot hash(proposal_id, 6): executed
slot hash(voter, hash(proposal_id, 7)): has_voted
slot hash(voter, hash(proposal_id, 8)): vote_weight
```

### Proposal Lifecycle

```
Created (submitted by member)
    ↓
Active (voting period)
    ↓
Passed (yes > no && quorum met)
    OR
Failed (deadline passed, quorum not met)
    ↓
Executed (if passed)
```

### Key Functions

- `createProposal(description, target, calldata)` → Submit proposal
- `vote(proposalId, support)` → Vote yes/no
- `executeProposal(proposalId)` → Execute if passed
- `getProposalState(proposalId)` → Check status
- `getVotes(address)` → Get voting power
- `delegate(to)` → Delegate voting power

### CLI Usage Example

```bash
# Deploy DAO with 50% quorum
minichain deploy --from alice --source dao.asm

# Proposal 1: "Increase block time to 10 seconds"
minichain call --from alice --to DAO --data <create_proposal>
minichain block produce --authority authority_0

# Members vote (assume Alice=40%, Bob=35%, Charlie=25% of tokens)
minichain call --from alice --to DAO --data <vote_yes>  # 40% yes
minichain block produce --authority authority_0

minichain call --from bob --to DAO --data <vote_yes>    # 35% yes
minichain block produce --authority authority_0

minichain call --from charlie --to DAO --data <vote_no> # 25% no
minichain block produce --authority authority_0

# Total: 75% yes, 25% no → Passed (quorum 50% met)

# Anyone can execute after voting period
minichain call --from anyone --to DAO --data <execute>
minichain block produce --authority authority_0

# Proposal executed!
```

### Use Cases

- Protocol governance
- Treasury management decisions
- Parameter updates (fees, limits)
- Upgrade proposals
- Community fund allocation

### Implementation Considerations

- Voting power based on token balance at proposal creation
- One vote per address per proposal
- Quorum calculated as percentage of total supply
- Prevent double voting
- Prevent execution before deadline
- Prevent execution if failed
- Snapshot balances at proposal creation

---

## Implementation Progression Path

We recommend implementing in this order:

### Phase 1: Foundation
**Escrow Contract** (120-150 lines)
- Learn state machines
- Handle three parties
- Master time-based logic
- Understand fund custody

### Phase 2: Economic Mechanics
**English Auction** (150-180 lines)
- Add competitive dynamics
- Implement refund patterns
- Handle bidding logic
- Master economic incentives

### Phase 3: Security Patterns
**Multi-Signature Wallet** (200-250 lines)
- Implement access control
- Handle proposal approval
- Master threshold logic
- Understand security patterns

### Phase 4: Token Standards
**ERC-20 Token** (150-200 lines)
- Implement mapping patterns
- Handle allowances
- Master balance tracking
- Foundation for DeFi

### Phase 5: Governance
**DAO Voting** (180-220 lines)
- Combine all previous patterns
- Implement weighted voting
- Master proposal execution
- Build governance primitive

---

## What Makes These "Non-Trivial"?

Unlike a simple counter, these contracts demonstrate:

✅ **Multiple storage slots** with complex patterns
✅ **Access control** (who can call what)
✅ **State machines** (proper transitions)
✅ **Multi-party interactions** (not just caller)
✅ **Error handling** (revert on invalid state)
✅ **Economic logic** (funds handling, refunds)
✅ **Time-based logic** (deadlines, expiration)
✅ **Security considerations** (re-entrancy, overflow)

---

## Testing Strategy

For each contract, create tests that cover:

1. **Happy Path**
   - Normal flow from start to finish
   - All parties behave correctly

2. **Access Control**
   - Unauthorized users cannot call restricted functions
   - Only designated roles can perform actions

3. **Edge Cases**
   - Boundary conditions (zero amounts, max values)
   - Timing issues (before deadline, after deadline)
   - State transitions (invalid state changes)

4. **Security**
   - Cannot re-enter
   - Cannot overflow/underflow
   - Cannot bypass checks

5. **Economic Correctness**
   - Funds are conserved
   - Refunds work correctly
   - No funds locked permanently

---

## Assembly Implementation Tips

### Storage Key Hashing

For mapping-like storage (address → value):
```asm
; Hash address with slot ID to get storage key
LOADI R0, <slot_id>
; R1 contains address
HASH R2, R0, R1      ; R2 = hash(slot_id || address)
SLOAD R3, R2         ; Load value from hashed key
```

### Access Control Pattern

```asm
; Check if caller is owner
CALLER R0            ; Get msg.sender
LOADI R1, 0          ; owner slot
SLOAD R2, R1         ; Load owner address
EQ R3, R0, R2        ; Compare
JUMPI R3, authorized
REVERT               ; Not authorized

authorized:
; Continue...
```

### State Machine Pattern

```asm
; Check current state
LOADI R0, 5          ; state slot
SLOAD R1, R0         ; Load state
LOADI R2, 1          ; Expected state (Funded)
EQ R3, R1, R2
JUMPI R3, valid_state
REVERT

valid_state:
; Perform action
; Update state
LOADI R4, 2          ; New state (Released)
SSTORE R0, R4
```

### Timestamp Checking

```asm
; Check if deadline passed
TIMESTAMP R0         ; Get current timestamp
LOADI R1, 4          ; deadline slot
SLOAD R2, R1         ; Load deadline
GT R3, R0, R2        ; current > deadline?
JUMPI R3, after_deadline
REVERT

after_deadline:
; Continue...
```

---

## Next Steps

1. **Choose a contract** (recommend starting with Escrow)
2. **Design the storage layout** (what goes in which slots)
3. **Write assembly code** (start with core functions)
4. **Test thoroughly** (happy path, edge cases, security)
5. **Document calldata format** (how to encode function calls)
6. **Create CLI scripts** (automate common workflows)
7. **Write tests** (unit tests in Rust)

Happy building! 🚀
