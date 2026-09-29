# Baseline: Vec-Based OrderBook

## Overview

The initial `OrderBook` implementation stores bids and asks using:

```rust
Vec<Order>
```

The structure is:

```text
OrderBook
├── bids: Vec<Order>
└── asks: Vec<Order>
```

This implementation is intentionally simple and serves as the performance baseline before introducing a more specialized order-book data structure.

The purpose of this benchmark is not to prove that `Vec<Order>` is bad, but to measure its current behavior and identify operations that become more expensive as the number of orders increases.

---

## Benchmark Tool

The benchmarks are implemented using Criterion.

They are executed with:

```bash
cargo bench --bench orderbook_benchmark
```

The benchmark uses Rust's standard `black_box`:

```rust
use std::hint::black_box;
```

This prevents the compiler from optimizing away the operation being measured.

---

## Current OrderBook Design

The current order book contains:

```rust
pub struct OrderBook {
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
}
```

Orders are stored directly in vectors.

Conceptually:

```text
Bids
┌────────┬────────┬────────┬────────┐
│ Order  │ Order  │ Order  │ Order  │
└────────┴────────┴────────┴────────┘

Asks
┌────────┬────────┬────────┬────────┐
│ Order  │ Order  │ Order  │ Order  │
└────────┴────────┴────────┴────────┘
```

The vectors provide contiguous memory storage and are simple to work with.

However, the current implementation does not maintain orders in price-sorted order.

Therefore, finding the best price requires scanning the vector.

---

## Benchmark Workloads

The following order-book sizes were tested:

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

This means the benchmark measures the operation itself rather than including the cost of constructing the order book.

---

# Results

## `best_bid()`

The current implementation searches through the bid vector to find the highest-priced order.

Measured results:

| Orders | Time |
|---:|---:|
| 1,000 | ~0.777 µs |
| 10,000 | ~7.302 µs |
| 100,000 | ~82.291 µs |

Criterion output:

```text
best_bid_1000
[769.36 ns 776.99 ns 784.26 ns]

best_bid_10000
[7.2402 µs 7.3024 µs 7.3623 µs]

best_bid_100000
[81.487 µs 82.291 µs 83.075 µs]
```

### Complexity

The implementation uses:

```rust
self.bids
    .iter()
    .enumerate()
    .max_by(...)
```

Therefore, every order may need to be inspected.

Expected complexity:

```text
O(n)
```

where `n` is the number of bid orders.

### Observation

As the number of orders increases from:

```text
1,000 → 10,000 → 100,000
```

the runtime increases approximately linearly.

This is consistent with the expected `O(n)` behavior.

---

## `best_ask()`

The current implementation searches through the ask vector to find the lowest-priced order.

Measured results:

| Orders | Time |
|---:|---:|
| 1,000 | ~0.720 µs |
| 10,000 | ~7.251 µs |
| 100,000 | ~80.796 µs |

Criterion output:

```text
best_ask_1000
[714.05 ns 719.80 ns 725.60 ns]

best_ask_10000
[7.1057 µs 7.2508 µs 7.4366 µs]

best_ask_100000
[79.639 µs 80.796 µs 82.371 µs]
```

### Complexity

The implementation searches the complete ask vector:

```rust
self.asks
    .iter()
    .enumerate()
    .min_by(...)
```

Expected complexity:

```text
O(n)
```

where `n` is the number of ask orders.

### Observation

The runtime again increases approximately linearly with the number of orders.

This matches the expected complexity of scanning a vector to discover the minimum price.

---

## `can_match()`

The current implementation determines whether the best bid is greater than or equal to the best ask.

Conceptually:

```text
can_match()
    │
    ├── best_bid()
    │
    └── best_ask()
```

The implementation is:

```rust
pub fn can_match(&self) -> bool {
    match (self.best_bid(), self.best_ask()) {
        (Some(bid_index), Some(ask_index)) => {
            self.bids[bid_index].price >= self.asks[ask_index].price
        }
        _ => false,
    }
}
```

Both sides contain the corresponding number of orders in this benchmark.

Measured results:

| Orders per side | Time |
|---:|---:|
| 1,000 | ~1.723 µs |
| 10,000 | ~15.337 µs |
| 100,000 | ~196.38 µs |

Criterion output:

```text
can_match_1000
[1.7036 µs 1.7231 µs 1.7409 µs]

can_match_10000
[15.216 µs 15.337 µs 15.479 µs]

can_match_100000
[189.25 µs 196.38 µs 203.96 µs]
```

### Complexity

`can_match()` performs:

```text
best_bid() → O(n)
best_ask() → O(n)
```

Therefore:

```text
O(n) + O(n)
```

which simplifies to:

```text
O(n)
```

The important point is that the two scans are performed sequentially.

They are not running in parallel.

---

# Summary of Current Complexity

| Operation | Current Data Structure | Expected Complexity |
|---|---|---:|
| Add order | `Vec<Order>` | O(n) currently* |
| Best bid | `Vec<Order>` | O(n) |
| Best ask | `Vec<Order>` | O(n) |
| Can match | `Vec<Order>` | O(n) |
| Cancel order | `Vec<Order>` | O(n) |

* `Vec::push()` itself is amortized `O(1)`, but the current `add_order()` first scans both vectors to detect duplicate order IDs, making the complete operation `O(n)`.

---

# Why `Vec<Order>` Becomes a Bottleneck

The main problem is not that `Vec` is inherently slow.

`Vec` has useful properties:

- contiguous memory
- good cache locality
- simple ownership model
- efficient iteration
- amortized `O(1)` append

The problem is that the current order-book operations require searching the vectors.

For example:

```rust
best_bid()
```

must examine the orders to determine which one has the highest price.

With:

```text
100 orders
```

the search may inspect 100 orders.

With:

```text
100,000 orders
```

the search may inspect 100,000 orders.

Therefore, the amount of work grows with the size of the order book.

---

# Why This Matters for a Matching Engine

A matching engine repeatedly needs to answer questions such as:

```text
What is the best bid?
What is the best ask?
Can the incoming order match?
Which order should execute first?
What is the next price level?
```

These operations are on the critical path of order matching.

With the current representation, price discovery requires scanning the order vectors.

For a small educational order book this is completely acceptable.

For a high-throughput matching engine, repeatedly scanning large vectors becomes increasingly expensive.

The benchmark gives us measurable evidence of this behavior.

---

# Important Benchmarking Observation

The benchmark results should be treated as a baseline rather than universal performance numbers.

The exact timings depend on factors such as:

- CPU
- operating system
- compiler version
- Rust optimization settings
- system load
- Criterion configuration
- machine architecture

Therefore, the important information is not simply:

```text
best_bid = 82.291 µs
```

but rather:

```text
best_bid scales approximately linearly as the order-book size increases.
```

The benchmark establishes a reference point that can be compared against future implementations on the same machine and methodology.

---

# Why We Don't Benchmark `cancel_order()` This Way

`cancel_order()` mutates the order book.

A naive benchmark such as:

```rust
b.iter(|| {
    book.cancel_order(order_id);
});
```

would not produce a meaningful repeated workload.

The first iteration would remove the order.

Subsequent iterations would attempt to cancel an order that no longer exists.

That means the benchmark would measure different operations after the first iteration.

For this reason, we are not adding `Clone` to `OrderBook` solely for benchmarking purposes.

Instead, cancellation performance will be evaluated as part of the redesigned order-book workload.

---

# Baseline Conclusion

The current `Vec<Order>` implementation provides a simple and cache-friendly foundation for the matching engine.

However, price discovery currently requires linear scans:

```text
best_bid() → O(n)
best_ask() → O(n)
can_match() → O(n)
```

The benchmark results demonstrate approximately linear growth in runtime as the number of orders increases.

This identifies price discovery as an important optimization target.

The benchmark therefore provides a quantitative baseline for evaluating alternative order-book data structures.

---

# Next Experiment

The next implementation will investigate a price-ordered structure:

```text
BTreeMap<Price, VecDeque<Order>>
```

Conceptually:

```text
OrderBook
│
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
FIFO order at the same price
```

This design is intended to better represent the requirements of a price-time-priority order book.

A secondary index for order IDs will also need to be considered because a price-ordered structure alone does not provide efficient lookup by `order_id`.

---

# Comparison Method

The next benchmark should use the same:

```text
1,000 orders
10,000 orders
100,000 orders
```

and the same operations wherever applicable.

The goal will be to compare:

```text
Vec<Order>
        vs
BTreeMap<Price, VecDeque<Order>>
```

rather than simply replacing one data structure because it appears theoretically better.

The comparison should consider:

- best-price lookup
- order insertion
- cancellation
- matching
- memory behavior
- implementation complexity
- price-time priority
- overall matching-engine workload

The benchmark results will determine whether the new structure actually improves the operations that matter.

---

