# Rust Order Book & Matching Engine

This is an event-driven order book and matching engine written
in Rust. The project explores price-time priority, exact fixed-point
arithmetic, order indexing, asynchronous command processing, and
lifecycle management.

> **Status:** Work in progress. A durable write-ahead log (WAL), crash
> recovery, and replay are planned, not yet implemented.

## Goals

-   Implement a limit order book with price-time priority.
-   Use integer-backed `Price` and `Quantity` types rather than
    floating-point arithmetic for financial values.
-   Support order insertion, cancellation, matching, and trade
    generation.
-   Keep order-book state owned by a single processor, coordinated
    through message passing.
-   Expose an asynchronous WebSocket interface.
-   Build confidence through unit and integration tests.

## System design

``` mermaid
flowchart TD
    C[WebSocket Client] -->|JSON order request| WS[WebSocket Handler]
    WS -->|Validate decimal strings| CMD[Order Command]
    CMD -->|Bounded Tokio MPSC queue| P[Single-owner Processor]
    P --> E[Matching Engine]
    E --> S[Price-Time Strategy]
    E --> B[Order Book]
    B --> BID[Bid levels: BTreeMap]
    B --> ASK[Ask levels: BTreeMap]
    B --> IDX[Order ID index: HashMap]
    E --> T[Trade result]
    T --> P
    P -->|Response via oneshot| WS
    WS -->|JSON response| C
    P -. planned .-> WAL[Append-only WAL]
    WAL -. planned replay .-> REC[Recovery / Replay]
    REC -. planned .-> E
```

### Main components

-   **WebSocket handler:** Converts incoming decimal strings into
    validated domain types.
-   **Command queue:** A bounded Tokio MPSC channel provides
    asynchronous message passing and backpressure.
-   **Processor:** Owns and mutates the matching engine, serializing
    order-book state transitions.
-   **Matching engine:** Coordinates order processing and the configured
    matching strategy.
-   **Price-time strategy:** Applies price priority first and FIFO time
    priority within a price level.
-   **Order book:**
    -   `BTreeMap<Price, VecDeque<Order>>` for bid and ask price levels.
    -   `VecDeque` preserves FIFO order within each price level.
    -   `HashMap<u64, OrderLocation>` indexes order IDs for cancellation
        lookup.
-   **Fixed-point types:** `Price` and `Quantity` use scaled integer
    units to avoid floating-point comparison and subtraction.
-   **WAL / replay (planned):** A future append-only log will record
    ordered state-changing commands and rebuild state after restart.

## Matching model

-   **Best bid:** Highest buy price.
-   **Best ask:** Lowest sell price.
-   **Crossing condition:** A buy order can match when its price is
    greater than or equal to the best ask.
-   **Price-time priority:** Better prices execute first; orders at the
    same price are processed FIFO.
-   **Trade price:** The current implementation uses the resting sell
    order's price when constructing a trade. Confirm this against your
    intended market rules before treating it as permanent policy.
-   **Partial fills:** Remaining quantity stays in the price-level
    queue; fully filled orders are removed from the book and index.

## Exact numeric types

`Price` and `Quantity` are integer-backed fixed-point types with a scale
of `1_000_000`, supporting up to six decimal places.

-   `"100.25"` is represented internally as `100_250_000` units.
-   `"5.25"` is represented internally as `5_250_000` units.

The API uses decimal strings so clients do not introduce binary
floating-point rounding before parsing. Values must be positive and have
at most six fractional decimal places.

**Tick size is separate from precision.** Six-decimal precision does not
mean every market allows every possible increment. Market-specific
tick-size validation can be added separately.

## Tech stack

-   **Rust**
-   **Tokio** --- asynchronous runtime and channels
-   **tokio-tungstenite** --- WebSocket transport
-   **Serde** --- serialization and deserialization
-   **BTreeMap / VecDeque / HashMap** --- order-book data structures

Exact dependencies are defined in `Cargo.toml`.

## Getting started

### Prerequisites

-   Rust toolchain installed through `rustup`
-   Cargo

### Build and test

``` bash
cargo fmt --check
cargo check
cargo test
```

### Run the server

Check the entry point and port configuration in `src/main.rs`, then run:

``` bash
cargo run
```

Document the WebSocket address and response schema here once they are
finalized in the implementation.

## Example order request

Represent financial decimals as strings:

``` json
{
  "id": 131,
  "price": "100.01",
  "quantity": "5.25",
  "side": "Buy"
}
```

This shows the intended request shape; use the actual endpoint and
response format configured by the current implementation.

## Testing priorities

-   Parsing and formatting `Price` and `Quantity`.
-   Rejection of zero, negative, excessive-precision, and out-of-range
    values.
-   Exact numeric ordering and subtraction.
-   Adding and cancelling orders.
-   Duplicate order IDs.
-   Best-bid / best-ask selection.
-   Price-time priority.
-   Full and partial fills.
-   Order-index consistency after cancellation and matching.
-   Bounded-queue backpressure and graceful shutdown.
-   **Planned:** WAL replay reconstructs identical book state and
    sequence IDs.
-   **Planned:** Recovery safely handles a truncated or corrupted final
    record.


## Design principles

1.  **Single ownership of mutable book state:** serialize state
    transitions through the processor.
2.  **Message passing before shared mutation:** coordinate tasks through
    channels where practical.
3.  **Exact financial arithmetic:** use fixed-point types for prices and
    quantities.
4.  **Explicit invariants:** keep the order book and order index
    consistent through state changes.
5.  **Correctness before optimization:** test behavior, then benchmark
    before optimizing.
6.  **Deterministic recovery:** define log ordering, record format,
    failure behavior, and durability semantics before implementing a
    WAL.



## Contributing

Keep contributions small and focused. Before submitting a change, run:

``` bash
cargo fmt --check
cargo check
cargo test
```

When changing matching behavior, add tests for the expected result and
the relevant order-book invariants.
