# BTreeMap-Based OrderBook Benchmark

## Overview

The initial `OrderBook` implementation used:

```rust
Vec<Order>
```

as the primary storage structure.

The next implementation migrated the order book to:

```rust
BTreeMap<Price, VecDeque<Order>>
```

The purpose of this benchmark is to compare the price-discovery operations against the original `Vec<Order>` baseline.

The goal is not to claim that `BTreeMap` is universally faster than `Vec`, but to measure how the new representation behaves for the operations that are important to a price-time-priority matching engine.

---

## New OrderBook Design

The new structure is:

```rust
pub type PriceLevel = VecDeque<Order>;

pub struct OrderBook {
    pub bids: BTreeMap<Price, PriceLevel>,
    pub asks: BTreeMap<Price, PriceLevel>,
}
```

Conceptually:

```text
OrderBook

├── bids
│   └── BTreeMap<Price, VecDeque<Order>>
│
└── asks
    └── BTreeMap<Price, VecDeque<Order>>
```

The responsibilities are separated:

```text
BTreeMap
    ↓
Price ordering

VecDeque
    ↓
FIFO ordering within a price level
```

For bids, the highest price is the best price.

For asks, the lowest price is the best price.

---

## Price Representation

`f64` cannot be used directly as a `BTreeMap` key because it does not implement `Ord`.

A `Price` newtype was introduced:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(pub i64);
```

Prices are converted to a fixed-point representation:

```rust
pub fn from_f64(price: f64) -> Self {
    Self((price * 1_000_000.0).round() as i64)
}
```

This gives the BTreeMap an ordered integer-backed key.

---

## Benchmark Tool

The benchmarks use Criterion.

They are executed with:

```bash
cargo bench --bench orderbook_benchmark
```

The benchmark uses Rust's standard `black_box`:

```rust
use std::hint::black_box;
```

This prevents the compiler from optimizing away the measured operation.

---

## Benchmark Workloads

The same order-book sizes used for the original baseline were tested:

- 1,000 orders
- 10,000 orders
- 100,000 orders

For `best_bid()`:

```text
1,000 buy orders
10,000 buy orders
100,000 buy orders
```

For `best_ask()`:

```text
1,000 sell orders
10,000 sell orders
100,000 sell orders
```

For `can_match()`:

```text
1,000 bids + 1,000 asks
10,000 bids + 10,000 asks
100,000 bids + 100,000 asks
```

The order-book construction happens outside the measured `b.iter()` section for these read-only benchmarks.

Therefore, the measured results represent the lookup/matching check itself rather than construction cost.

---

# Results

## `best_bid()`

The new implementation uses:

```rust
pub fn best_bid(&self) -> Option<Price> {
    self.bids
        .last_key_value()
        .map(|(price, _)| *price)
}
```

Because bids are ordered by price, the last key is the highest bid.

### Measured results

| Orders | Time |
|---:|---:|
| 1,000 | ~2.4 ns |
| 10,000 | ~2.9 ns |
| 100,000 | ~3.7 ns |

### Complexity

`last_key_value()` directly accesses the boundary of the BTreeMap.

Therefore:

```text
best_bid() → O(1)
```

It does not scan all orders.

### Observation

The runtime remains approximately constant as the number of orders increases:

```text
1,000
   ↓
10,000
   ↓
100,000

~2.4 ns
~2.9 ns
~3.7 ns
```

This is consistent with the O(1) boundary lookup.

---

## `best_ask()`

The new implementation uses:

```rust
pub fn best_ask(&self) -> Option<Price> {
    self.asks
        .first_key_value()
        .map(|(price, _)| *price)
}
```

Because asks are ordered by price, the first key is the lowest ask.

### Measured results

| Orders | Time |
|---:|---:|
| 1,000 | ~2.7 ns |
| 10,000 | ~3.1 ns |
| 100,000 | ~4.1 ns |

### Complexity

`first_key_value()` directly accesses the first key.

Therefore:

```text
best_ask() → O(1)
```

### Observation

The runtime remains approximately constant as the order-book size increases.

---

## `can_match()`

The new implementation is:

```rust
pub fn can_match(&self) -> bool {
    match (self.best_bid(), self.best_ask()) {
        (Some(bid_price), Some(ask_price)) => {
            bid_price >= ask_price
        }
        _ => false,
    }
}
```

Conceptually:

```text
can_match()

    ↓

best_bid()
    +
best_ask()

    ↓

bid_price >= ask_price
```

### Measured results

| Orders per side | Time |
|---:|---:|
| 1,000 | ~4.5 ns |
| 10,000 | ~5.5 ns |
| 100,000 | ~8.0 ns |

### Complexity

Both `best_bid()` and `best_ask()` are O(1).

Therefore:

```text
can_match()
    = O(1) + O(1)
    = O(1)
```

The comparison itself is also O(1).

### Observation

The runtime remains approximately constant as the order-book size increases.

---

# Comparison With Vec Baseline

The original `Vec<Order>` benchmark produced:

### `best_bid()`

| Orders | Vec | BTreeMap |
|---:|---:|---:|
| 1,000 | ~0.777 µs | ~2.4 ns |
| 10,000 | ~7.302 µs | ~2.9 ns |
| 100,000 | ~82.291 µs | ~3.7 ns |

### `best_ask()`

| Orders | Vec | BTreeMap |
|---:|---:|---:|
| 1,000 | ~0.720 µs | ~2.7 ns |
| 10,000 | ~7.251 µs | ~3.1 ns |
| 100,000 | ~80.796 µs | ~4.1 ns |

### `can_match()`

| Orders per side | Vec | BTreeMap |
|---:|---:|---:|
| 1,000 | ~1.723 µs | ~4.5 ns |
| 10,000 | ~15.337 µs | ~5.5 ns |
| 100,000 | ~196.38 µs | ~8.0 ns |

The exact nanosecond values depend on hardware, compiler version, optimization settings, and system load.

The important result is the change in scaling behavior:

```text
Vec

N increases
    ↓
linear scan becomes more expensive


BTreeMap

N increases
    ↓
boundary lookup remains approximately constant
```

---

# Complexity Summary

Let:

- `N` = total number of orders
- `P` = number of price levels

| Operation | Vec Implementation | BTreeMap Implementation |
|---|---:|---:|
| Best bid | O(N) | **O(1)** |
| Best ask | O(N) | **O(1)** |
| Can match | O(N) | **O(1)** |
| Price insertion | O(1)* | O(log P) |
| Add order | O(N)** | O(N)*** |
| Cancel order | O(N) | O(N) |
| Execute trade | O(1) after indexes | O(log P) map access + O(1) queue operations |

\* `Vec::push()` is amortized O(1).

\** The current Vec implementation scans the book to detect duplicate order IDs.

\*** The BTreeMap price insertion is O(log P), but the current duplicate-ID scan is O(N), so the complete `add_order()` remains O(N).

The BTreeMap migration therefore primarily optimizes **price discovery**, not every operation.

---

# Price-Time Priority

The new representation directly models price-time priority.

For bids:

```text
105 → [Order 1, Order 4]
103 → [Order 2]
100 → [Order 3]

Best bid = 105
```

For asks:

```text
106 → [Order 5, Order 7]
108 → [Order 6]

Best ask = 106
```

At the same price:

```text
106 → [Order 5, Order 7]
       ↑
    executes first
```

`VecDeque` provides the FIFO behavior.

This makes the matching rule explicit:

```text
1. Best price
2. Earliest order at that price
```

---

# Matching Behavior

The matching strategy repeatedly obtains:

```rust
let bid_price = book.best_bid();
let ask_price = book.best_ask();
```

and executes the front orders at those price levels.

The basic flow is:

```text
Best bid
   ↓
Best ask
   ↓
Do prices cross?
   ↓
Execute front orders
   ↓
Remove filled orders
   ↓
Repeat
```

Partial fills are supported.

For example:

```text
Buy quantity  = 10
Sell quantity = 4
```

The trade quantity is:

```text
min(10, 4) = 4
```

After execution:

```text
Buy quantity  = 6
Sell quantity = 0
```

---

# Why Best Bid / Ask Are O(1)

A common misconception is that every BTreeMap lookup is O(log P).

A normal lookup such as:

```rust
book.bids.get(&price)
```

is O(log P).

However, the current best-price operations use:

```rust
first_key_value()
last_key_value()
```

These are boundary operations.

Therefore:

```text
best_bid() → O(1)
best_ask() → O(1)
```

The benchmark results also show approximately constant runtime as the number of orders grows.

---

# Cancellation

Cancellation is still implemented by scanning the price-level queues.

Therefore:

```text
cancel_order() → O(N)
```

The BTreeMap optimizes lookup by price, but it does not provide fast lookup by `OrderId`.

A separate order-ID index is therefore needed.

Conceptually:

```text
                 OrderBook
                /         \
         Price Index      ID Index
         BTreeMap         HashMap
             ↓               ↓
       price priority    order lookup
```

The next optimization is to add this secondary index.

---

# Benchmarking Notes

The benchmark numbers are reference measurements, not universal performance guarantees.

Actual timings depend on:

- CPU
- operating system
- compiler version
- Rust optimization settings
- machine architecture
- system load
- Criterion configuration

The most useful comparison is therefore the scaling behavior and the relative performance measured using the same machine and methodology.

The benchmark also showed that repeated Criterion runs can vary slightly at nanosecond scale. Small percentage changes at these timings should not automatically be interpreted as meaningful regressions.

---

# Cancellation Benchmark Note

A cancellation benchmark was attempted using `iter_batched()`.

The observed results were approximately:

```text
cancel_order_1000    ~28.8 µs
cancel_order_10000  ~509.9 µs
cancel_order_100000 did not finish
```

These numbers are **not** used as the formal cancellation baseline.

The benchmark included order-book construction in the batch setup, and the current `add_order()` performs an O(N) duplicate-ID scan for each insertion.

Therefore, the measured time does not isolate cancellation itself.

A proper cancellation benchmark should separate:

```text
book construction
        ↓
cancellation measurement
```

before using the results for comparison.

---

# What Improved

The migration changed the main price-discovery operations from:

```text
best_bid()  → O(N)
best_ask()  → O(N)
can_match() → O(N)
```

to:

```text
best_bid()  → O(1)
best_ask()  → O(1)
can_match() → O(1)
```

The benchmark reflects this change.

The BTreeMap is therefore a better representation for the price-level access pattern of the matching engine.

This does not mean the BTreeMap is optimal for every operation.

---

# What Still Needs Optimization

The current implementation still has O(N) work in:

```text
add_order()
cancel_order()
```

The next optimization is a secondary order-ID index:

```text
HashMap<OrderId, ...>
```

This will allow the engine to locate an order without scanning the entire order book.

Future work will also consider:

- exact fixed-point price representation
- exact quantity representation
- order-ID indexing
- cancellation performance
- matching throughput
- memory behavior
- hot-path optimization
- atomics and memory ordering
- advanced Tokio/concurrency
- persistence and production architecture

---

# Conclusion

The `BTreeMap<Price, VecDeque<Order>>` migration changes the order book from an unsorted collection of orders into a structure that directly represents price-time priority.

```text
BTreeMap
    ↓
Price priority

VecDeque
    ↓
Time priority
```

The most important measured improvement is price discovery:

```text
best_bid()  → O(1)
best_ask()  → O(1)
can_match() → O(1)
```

The original Vec implementation required linear scans whose runtime increased substantially with order-book size.

The next experiment is to introduce a secondary `HashMap<OrderId, ...>` index to address the remaining O(N) cancellation and duplicate-ID lookup costs.
