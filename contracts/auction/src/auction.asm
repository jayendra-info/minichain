; ============================================================================
; English Auction Smart Contract
; ============================================================================
;
; DESCRIPTION
; -----------
; An English (ascending-price) auction where bidders compete by placing
; increasingly higher bids. When the auction period ends, the highest
; bidder wins the item and the seller can withdraw the proceeds.
;
; PARTICIPANTS
; ------------
;   Seller   – Deploys the contract; receives the winning bid.
;   Bidders  – Call `bid` with an amount higher than the current highest bid.
;              Outbid bidders can withdraw their refunds via `withdraw`.
;
; STORAGE LAYOUT
; --------------
;   Slot 0 : seller_address        – address of the auction creator
;   Slot 1 : highest_bid           – the current highest bid amount
;   Slot 2 : highest_bidder        – address of the current highest bidder
;   Slot 3 : auction_end_time      – timestamp when bidding closes
;   Slot 4 : reserve_price         – minimum acceptable bid
;   Slot 5 : ended                 – 1 if auction has been finalized, else 0
;   Slot 6 : min_bid_increment     – minimum amount a new bid must exceed
;                                     the current highest bid by
;   Slot 10+: pending_returns      – per-bidder refund ledger
;             Key = 10 + (caller_id mod 2^56) so each bidder gets a unique slot
;
; FUNCTION DISPATCH
; -----------------
; The contract uses CALLVALUE to differentiate between functions:
;   - CALLVALUE > 0          → bid()          Place a new bid
;   - CALLVALUE == 0, slot-based dispatch via caller role:
;       Caller == seller      → seller_withdraw() Seller claims winning bid
;       Caller != seller      → withdraw()        Outbid bidder reclaims funds
;
; Note: endAuction() is called automatically when any action detects the
;       auction period has elapsed and `ended` is still 0.
;
; WORKFLOW
; --------
;   1. Seller deploys.  On first run the initializer stores the seller
;      address, reserve price (100), end time (current timestamp + 3600),
;      and minimum bid increment (10).
;
;   2. Bidders call with CALLVALUE > 0.  If the bid exceeds
;      highest_bid + min_bid_increment AND is >= reserve_price AND
;      the auction has not ended, the previous highest bidder's amount is
;      credited to their pending_returns slot, and the new bid becomes the
;      highest.
;
;   3. After the end time, any transaction triggers finalization automatically.
;
;   4. The seller calls (CALLVALUE == 0) to withdraw the winning bid.
;      Outbid bidders call (CALLVALUE == 0) to withdraw their refunds.
;
; REGISTERS USED
; --------------
;   R0  – scratch / storage keys
;   R1  – scratch / storage values / loaded data
;   R2  – scratch / comparisons & arithmetic
;   R3  – scratch / comparison results / flags
;   R4  – caller address
;   R5  – call value (bid amount)
;   R6  – jump targets
;   R7  – current timestamp
;   R8  – scratch for pending returns key / value
;   R9  – scratch for arithmetic
;   R10 – scratch
;   R11 – scratch
;
; ============================================================================

.entry main

; --------------------------------------------------------------------------
; CONSTANTS  (used as storage slot keys)
; --------------------------------------------------------------------------
.const SLOT_SELLER           0
.const SLOT_HIGHEST_BID      1
.const SLOT_HIGHEST_BIDDER   2
.const SLOT_END_TIME         3
.const SLOT_RESERVE_PRICE    4
.const SLOT_ENDED            5
.const SLOT_MIN_INCREMENT    6
.const PENDING_RETURNS_BASE  10

; ============================================================================
; ENTRY POINT
; ============================================================================
main:
    ; -- Load context --
    CALLER   R4              ; R4 = msg.sender
    CALLVALUE R5             ; R5 = msg.value (bid amount)
    TIMESTAMP R7             ; R7 = current block timestamp

    ; -- Check if contract is already initialized (seller != 0) --
    LOADI    R0, 0           ; slot 0 = seller
    SLOAD    R1, R0          ; R1 = seller_address
    LOADI    R2, 0
    EQ       R3, R1, R2      ; R3 = 1 if seller == 0 (not initialized)
    LOADI    R6, init
    JUMPI    R3, R6          ; if not initialized → init

    ; -- Auto-finalize: check if auction time has elapsed --
    LOADI    R0, 5           ; slot 5 = ended
    SLOAD    R1, R0          ; R1 = ended flag
    LOADI    R2, 0
    EQ       R3, R1, R2      ; R3 = 1 if ended == 0
    LOADI    R6, check_time
    JUMPI    R3, R6          ; if not ended yet → check_time
    ; Auction already ended, skip time check
    LOADI    R6, dispatch
    LOADI    R3, 1
    JUMPI    R3, R6          ; unconditional jump → dispatch

check_time:
    ; Check if current timestamp >= auction_end_time
    LOADI    R0, 3           ; slot 3 = auction_end_time
    SLOAD    R1, R0          ; R1 = end_time
    GE       R3, R7, R1      ; R3 = 1 if now >= end_time
    LOADI    R6, do_end
    JUMPI    R3, R6          ; if time elapsed → do_end
    ; Auction still active, proceed to dispatch
    LOADI    R6, dispatch
    LOADI    R3, 1
    JUMPI    R3, R6          ; unconditional jump → dispatch

; --------------------------------------------------------------------------
; AUTO-FINALIZE: mark ended = 1
; --------------------------------------------------------------------------
do_end:
    LOADI    R0, 5           ; slot 5 = ended
    LOADI    R1, 1
    SSTORE   R0, R1          ; ended = 1
    LOG      R1              ; log: auction ended (1)
    ; fall through to dispatch

; ============================================================================
; DISPATCH – route based on CALLVALUE
; ============================================================================
dispatch:
    LOADI    R2, 0
    GT       R3, R5, R2      ; R3 = 1 if callvalue > 0
    LOADI    R6, bid
    JUMPI    R3, R6          ; if value sent → bid()

    ; CALLVALUE == 0: check if caller is seller
    LOADI    R0, 0           ; slot 0 = seller
    SLOAD    R1, R0          ; R1 = seller_address
    EQ       R3, R4, R1      ; R3 = 1 if caller == seller
    LOADI    R6, seller_withdraw
    JUMPI    R3, R6          ; if seller → seller_withdraw()

    ; Otherwise caller is a bidder wanting refund
    LOADI    R6, withdraw
    LOADI    R3, 1
    JUMPI    R3, R6          ; unconditional jump → withdraw()

; ============================================================================
; INIT – First-time setup (called on deployment)
; ============================================================================
;   Sets: seller = caller,  reserve_price = 100,
;         auction_end_time = now + 3600,  min_bid_increment = 10
; ============================================================================
init:
    ; seller = caller
    LOADI    R0, 0           ; slot 0
    SSTORE   R0, R4          ; seller = msg.sender

    ; reserve_price = 100
    LOADI    R0, 4           ; slot 4
    LOADI    R1, 100
    SSTORE   R0, R1

    ; auction_end_time = now + 3600 (1 hour)
    LOADI    R0, 3           ; slot 3
    LOADI    R2, 3600
    ADD      R1, R7, R2      ; end_time = timestamp + 3600
    SSTORE   R0, R1

    ; min_bid_increment = 10
    LOADI    R0, 6           ; slot 6
    LOADI    R1, 10
    SSTORE   R0, R1

    ; highest_bid = 0 (already default)
    ; highest_bidder = 0 (already default)
    ; ended = 0 (already default)

    LOG      R4              ; log: contract initialized by seller
    HALT

; ============================================================================
; BID – Place a new bid
; ============================================================================
;   Requires:
;     - Auction has NOT ended (ended == 0)
;     - Bid >= reserve_price
;     - Bid >= highest_bid + min_bid_increment
;   Effects:
;     - Previous highest bidder's amount → pending_returns
;     - Updates highest_bid and highest_bidder
; ============================================================================
bid:
    ; -- Guard: auction must not be ended --
    LOADI    R0, 5           ; slot 5 = ended
    SLOAD    R1, R0
    LOADI    R2, 0
    NE       R3, R1, R2      ; R3 = 1 if ended != 0
    LOADI    R6, bid_fail
    JUMPI    R3, R6          ; if ended → revert

    ; -- Guard: bid >= reserve_price --
    LOADI    R0, 4           ; slot 4 = reserve_price
    SLOAD    R1, R0          ; R1 = reserve_price
    LT       R3, R5, R1      ; R3 = 1 if bid < reserve_price
    LOADI    R6, bid_fail
    JUMPI    R3, R6          ; if bid too low → revert

    ; -- Guard: bid >= highest_bid + min_bid_increment --
    LOADI    R0, 1           ; slot 1 = highest_bid
    SLOAD    R8, R0          ; R8 = highest_bid
    LOADI    R0, 6           ; slot 6 = min_bid_increment
    SLOAD    R9, R0          ; R9 = min_bid_increment
    ADD      R10, R8, R9     ; R10 = highest_bid + min_increment
    LT       R3, R5, R10     ; R3 = 1 if bid < required minimum
    LOADI    R6, bid_fail
    JUMPI    R3, R6          ; if bid not enough → revert

    ; -- Credit previous highest bidder's pending return --
    ; Only if there IS a previous highest bidder (highest_bid > 0)
    LOADI    R2, 0
    GT       R3, R8, R2      ; R3 = 1 if highest_bid > 0
    LOADI    R6, credit_prev
    JUMPI    R3, R6          ; if previous bid exists → credit it
    ; No previous bidder, skip crediting
    LOADI    R6, update_bid
    LOADI    R3, 1
    JUMPI    R3, R6          ; unconditional jump → update_bid

credit_prev:
    ; Load previous highest bidder address
    LOADI    R0, 2           ; slot 2 = highest_bidder
    SLOAD    R11, R0         ; R11 = prev_highest_bidder

    ; Compute pending_returns key for prev bidder
    ; key = PENDING_RETURNS_BASE + (bidder_addr mod 2^56)
    LOADI    R9, 0x00FFFFFFFFFFFFFF   ; mask for lower 56 bits
    AND      R10, R11, R9    ; R10 = bidder_addr mod 2^56
    LOADI    R9, 10          ; PENDING_RETURNS_BASE
    ADD      R0, R9, R10     ; R0 = storage slot key

    ; Load existing pending return and add previous bid
    SLOAD    R1, R0          ; R1 = current pending return
    ADD      R1, R1, R8      ; R1 += previous highest_bid
    SSTORE   R0, R1          ; save updated pending return

    LOG      R11             ; log: credited refund to prev bidder

update_bid:
    ; -- Update highest_bid = callvalue --
    LOADI    R0, 1           ; slot 1 = highest_bid
    SSTORE   R0, R5          ; highest_bid = msg.value

    ; -- Update highest_bidder = caller --
    LOADI    R0, 2           ; slot 2 = highest_bidder
    SSTORE   R0, R4          ; highest_bidder = msg.sender

    LOG      R5              ; log: new highest bid amount
    LOG      R4              ; log: new highest bidder address
    HALT

bid_fail:
    ; Bid validation failed – revert the transaction
    REVERT

; ============================================================================
; WITHDRAW – Outbid bidder reclaims their funds
; ============================================================================
;   Lookup caller's pending_returns slot, log the amount, zero it out.
;   (Actual token/balance transfer is handled by the chain executor layer)
; ============================================================================
withdraw:
    ; Compute pending_returns key for caller
    LOADI    R9, 0x00FFFFFFFFFFFFFF
    AND      R10, R4, R9     ; R10 = caller mod 2^56
    LOADI    R9, 10
    ADD      R0, R9, R10     ; R0 = storage key

    ; Load pending return
    SLOAD    R1, R0          ; R1 = amount owed

    ; Guard: must have something to withdraw
    LOADI    R2, 0
    EQ       R3, R1, R2      ; R3 = 1 if amount == 0
    LOADI    R6, withdraw_fail
    JUMPI    R3, R6          ; nothing to withdraw → revert

    ; Clear the pending return (prevent re-entrancy)
    LOADI    R2, 0
    SSTORE   R0, R2          ; pending_returns[caller] = 0

    ; Log the withdrawal amount
    LOG      R1              ; log: withdrawn amount
    LOG      R4              ; log: withdrawer address
    HALT

withdraw_fail:
    REVERT

; ============================================================================
; SELLER_WITHDRAW – Seller claims the winning bid
; ============================================================================
;   Requires:
;     - Auction has ended (ended == 1)
;     - highest_bid > 0  (there was at least one valid bid)
;   Effects:
;     - Logs the winning amount for the chain layer to transfer
;     - Zeros out highest_bid to prevent double-withdraw
; ============================================================================
seller_withdraw:
    ; -- Guard: auction must have ended --
    LOADI    R0, 5           ; slot 5 = ended
    SLOAD    R1, R0
    LOADI    R2, 1
    NE       R3, R1, R2      ; R3 = 1 if ended != 1
    LOADI    R6, seller_fail
    JUMPI    R3, R6          ; if not ended → revert

    ; -- Guard: highest_bid > 0 --
    LOADI    R0, 1           ; slot 1 = highest_bid
    SLOAD    R1, R0          ; R1 = winning amount
    LOADI    R2, 0
    EQ       R3, R1, R2      ; R3 = 1 if highest_bid == 0
    LOADI    R6, seller_fail
    JUMPI    R3, R6          ; nothing to claim → revert

    ; -- Clear highest bid to prevent double withdrawal --
    LOADI    R2, 0
    SSTORE   R0, R2          ; highest_bid = 0

    ; -- Log: seller receives the winning bid --
    LOG      R1              ; log: winning bid amount
    LOG      R4              ; log: seller address
    HALT

seller_fail:
    REVERT
