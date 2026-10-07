# HashMap Order Index Optimization

## Overview

The initial order book used `Vec<Order>` for storing bids and asks.

The first major data-structure migration replaced the vectors with:

```text
BTreeMap<Price, VecDeque<Order>>
```

This solved the problem of efficiently maintaining price levels and finding the best bid and ask.

However, a new problem remained.

A matching engine frequently needs to locate an order by its `order_id`, especially for operations such as:

```text
Cancel order
Modify order
Check order status
Remove filled order
```

A `BTreeMap` is organized by price, not by order ID.

Therefore, even after the BTreeMap migration, locating an arbitrary order by ID could still require searching through the order book.

To solve this, a secondary `HashMap` index was introduced.

The resulting structure is:

```text
OrderBook

├── bids
│   └── BTreeMap<Price, VecDeque<Order>>
│
├── asks
│   └── BTreeMap<Price, VecDeque<Order>>
│
└── order_index
    └── HashMap<u64, OrderLocation>
```

The purpose of this migration is to provide efficient order-ID lookup without sacrificing the price-ordered structure required for matching.

---

# Why the HashMap Index Was Needed

The BTreeMap organizes orders according to:

```text
Price
  ↓
Orders at that price
```

For example:

```text
Bids

101.0 → [Order 8, Order 12]
100.0 → [Order 3, Order 7, Order 10]
99.0  → [Order 4]
```

This is excellent for questions such as:

```text
What is the highest bid?
What is the lowest ask?
Which price level should match next?
```

But cancellation starts with:

```text
order_id = 10
```

The BTreeMap does not directly answer:

```text
Where is order 10?
```

Without an additional index, the engine would have to search through price levels and orders.

Conceptually:

```text
order_id
   ↓
search bids
   ↓
search asks
   ↓
search orders
   ↓
find matching order
```

With `N` total orders, this can become:

```text
O(N)
```

The HashMap provides a direct mapping:

```text
Order ID
    ↓
OrderLocation
```

---

# New OrderLocation Structure

A new structure was introduced:

```rust
#[derive(Debug, Clone, Copy)]
pub struct OrderLocation {
    pub side: Side,
    pub price: Price,
}
```

The index is:

```rust
HashMap<u64, OrderLocation>
```

where:

```text
u64
 ↓
order_id
```

and:

```text
OrderLocation
├── side
└── price
```

For example:

```text
Order ID 42

        ↓

HashMap

        ↓

OrderLocation {
    side: Buy,
    price: Price(100_000_000)
}
```

The HashMap does not duplicate the complete `Order`.

It only stores its location.

---

# Updated OrderBook Structure

The complete structure is now:

```rust
pub struct OrderBook {
    pub bids: BTreeMap<Price, PriceLevel>,
    pub asks: BTreeMap<Price, PriceLevel>,
    pub order_index: HashMap<u64, OrderLocation>,
}
```

where:

```rust
pub type PriceLevel = VecDeque<Order>;
```

Conceptually:

```text
                    OrderBook
                       │
          ┌────────────┼────────────┐
          │            │            │
          ▼            ▼            ▼
       BTreeMap     BTreeMap      HashMap
        Bids         Asks       Order Index
          │            │            │
          ▼            ▼            ▼
     Price levels  Price levels  Order ID
          │            │            │
          ▼            ▼            ▼
       VecDeque     VecDeque    Location
```

Each data structure now has a separate responsibility.

---

# Responsibilities of Each Data Structure

## BTreeMap

Responsible for:

```text
Price ordering
Best bid
Best ask
Price-level lookup
```

## VecDeque

Responsible for:

```text
FIFO ordering
Price-time priority
Efficient front removal
```

## HashMap

Responsible for:

```text
Order-ID lookup
Duplicate-ID detection
Finding side and price
```

---

# Add Order Changes

Previously, duplicate order IDs were detected by scanning the entire order book.

The new implementation uses:

```rust
self.order_index.contains_key(&order.id)
```

This gives average:

```text
O(1)
```

lookup for duplicate detection.

After the order is placed into its price level, the index is updated:

```text
Order
  │
  ├── BTreeMap
  │      ↓
  │   Price Level
  │
  └── HashMap
         ↓
      Location
```

Therefore the order is represented in two complementary structures.

---

# Cancellation Before the HashMap

The previous cancellation path had to search the order book.

Conceptually:

```text
cancel_order(order_id)
        │
        ▼
   Search bids
        │
        ▼
   Search asks
        │
        ▼
   Search orders
        │
        ▼
   Remove order
```

With `N` total orders, the search could inspect a large portion of the book.

Therefore:

```text
Cancel order → O(N)
```

---

# Cancellation After the HashMap

The new cancellation path is:

```text
order_id
    │
    ▼
HashMap lookup
    │
    ▼
OrderLocation
    │
    ├── Side
    └── Price
    │
    ▼
BTreeMap lookup
    │
    ▼
PriceLevel
    │
    ▼
Search order inside VecDeque
    │
    ▼
Remove order
```

This eliminates the need to search the entire order book.

---

# Current Cancellation Complexity

It is important not to describe the new cancellation operation as simply `O(1)`.

There are three components.

## 1. HashMap lookup

```rust
let location = self.order_index.get(&order_id).copied()?;
```

Average complexity:

```text
O(1)
```

## 2. BTreeMap lookup

The index gives us the price.

The corresponding price level is then retrieved from the BTreeMap.

Complexity:

```text
O(log P)
```

where:

```text
P = number of price levels
```

## 3. Search inside the price level

The current implementation uses:

```rust
queue.iter().position(|order| order.id == order_id)
```

If there are:

```text
K = number of orders at that price
```

then this search is:

```text
O(K)
```

Therefore the complete cancellation complexity is:

```text
O(1) + O(log P) + O(K)
```

which simplifies to:

```text
O(log P + K)
```

---

# Why Cancellation Is No Longer O(N)

The important difference is the meaning of `N` and `K`.

Previously:

```text
N = all orders in the book
```

and the cancellation search could inspect:

```text
N orders
```

Now:

```text
N = all orders in the book
P = number of price levels
K = orders at the target price
```

The HashMap immediately tells us the target price.

Therefore we only search:

```text
K orders
```

instead of:

```text
N orders
```

For example:

```text
100,000 total orders

Price 100:
    5 orders
```

The HashMap can take us directly to:

```text
Price 100
```

and the queue search only needs to inspect those 5 orders.

---

# Why O(K) Still Exists

The current price level is a `VecDeque<Order>`.

For example:

```text
Price 100

┌────────┬────────┬────────┬────────┬────────┐
│ Order1 │ Order2 │ Order3 │ Order4 │ Order5 │
└────────┴────────┴────────┴────────┴────────┘
```

Suppose we need:

```text
Cancel Order5
```

The current implementation does:

```rust
queue.iter().position(|order| order.id == order_id)
```

It checks:

```text
Order1 → no
Order2 → no
Order3 → no
Order4 → no
Order5 → yes
```

Therefore the search can require approximately `K` inspections.

This is the remaining linear component.

---

# Benchmarking Methodology

The benchmarks use Criterion.

The benchmark imports:

```rust
use std::hint::black_box;
```

`black_box()` prevents the compiler from optimizing away the operation being measured.

The benchmark is executed using:

```bash
cargo bench --bench orderbook_benchmark
```

For cancellation specifically:

```bash
cargo bench --bench orderbook_benchmark -- cancel_order
```

---

# Why Cancellation Requires a Special Benchmark

`cancel_order()` mutates the order book.

A naive benchmark such as:

```rust
b.iter(|| {
    book.cancel_order(order_id);
});
```

would not represent the same operation after the first iteration.

The first iteration would remove the order.

Later iterations would attempt to cancel an order that no longer exists.

Therefore the benchmark uses `iter_batched()`:

```rust
b.iter_batched(
    || {
        // Create fresh OrderBook
    },
    |mut book| {
        black_box(book.cancel_order(order_id));
    },
    BatchSize::SmallInput,
);
```

This creates a fresh workload for each measured cancellation.

---

# Benchmark 1: Cancellation With Increasing Book Size

The first cancellation benchmark used:

```text
1,000 orders
10,000 orders
100,000 orders
```

Each order was placed at a different price.

Therefore:

```text
N = total orders
P ≈ N
K = 1
```

This workload is useful for observing cancellation behavior as the book grows, but it does not isolate the `O(K)` queue-search component because each price level contains only one order.

---

# Benchmark 1 Results

The latest benchmark results were:

| Total Orders | Cancellation Time |
|---:|---:|
| 1,000 | ~19.86 µs |
| 10,000 | ~334.35 µs |
| 100,000 | ~3.14 ms |

Criterion output:

```text
cancel_order_1000
time: [19.366 µs 19.862 µs 20.300 µs]

change: [-35.365% -32.130% -29.064%]
Performance has improved.
```

For 10,000 orders:

```text
cancel_order_10000
time: [325.17 µs 334.35 µs 341.66 µs]

change: [-7.5155% -4.0923% -0.4459%]
Change within noise threshold.
```

For 100,000 orders:

```text
cancel_order_100000
time: [3.1217 ms 3.1441 ms 3.1675 ms]

change: [-21.931% -21.243% -20.552%]
Performance has improved.
```

The 1,000 and 100,000 order comparisons showed statistically significant improvement according to Criterion.

The 10,000 order comparison was within the configured noise threshold.

---

# Important Interpretation of Benchmark 1

The benchmark results increase as the book size increases.

However, this should not be interpreted as proof that `cancel_order()` is `O(N)`.

The implementation itself is:

```text
O(log P + K)
```

and in this workload:

```text
P ≈ N
K = 1
```

so the cancellation algorithm is approximately:

```text
O(log N)
```

The absolute benchmark numbers are also influenced by the complete benchmark workload and machine behavior.

Therefore the next benchmark was designed to isolate `K`.

---

# Benchmark 2: Isolating the Price-Level Search

To directly investigate the remaining `O(K)` component, the next benchmark kept the workload focused on a single price level.

The number of orders at the same price was changed:

```text
K = 1
K = 10
K = 100
K = 1,000
K = 10,000
```

All orders were placed at:

```text
Price = 100.0
```

The benchmark cancels the last order in the queue.

This is intentional.

If the benchmark cancelled the first order, `position()` could find it immediately.

By cancelling the last order, the queue search has to traverse approximately the entire price level.

---

# Benchmark 2 Results

The measured results were:

| Orders At Same Price (`K`) | Cancellation Time |
|---:|---:|
| 1 | ~90.62 ns |
| 10 | ~123.34 ns |
| 100 | ~273.35 ns |
| 1,000 | ~1.30 µs |
| 10,000 | ~16.14 µs |

Full Criterion output:

```text
cancel_order_queue_1
time: [87.859 ns 90.619 ns 94.688 ns]

Found 6 outliers among 100 measurements
6.00%

3 low mild
2 high mild
1 high severe
```

```text
cancel_order_queue_10
time: [121.45 ns 123.34 ns 124.97 ns]

Found 3 outliers among 100 measurements
3.00%

3 low mild
```

```text
cancel_order_queue_100
time: [259.76 ns 273.35 ns 285.33 ns]

Found 1 outlier among 100 measurements
1.00%

1 high mild
```

```text
cancel_order_queue_1000
time: [1.2486 µs 1.2996 µs 1.3452 µs]

Found 1 outlier among 100 measurements
1.00%

1 high mild
```

```text
cancel_order_queue_10000
time: [15.145 µs 16.137 µs 17.177 µs]
```

---

# Interpretation of Benchmark 2

The important observation is:

```text
K increases
     ↓
cancellation time increases
```

The results:

```text
K = 1       → ~90.62 ns
K = 10      → ~123.34 ns
K = 100     → ~273.35 ns
K = 1,000   → ~1.30 µs
K = 10,000  → ~16.14 µs
```

show the cost associated with searching within the price-level queue.

The relationship is not perfectly linear.

That is expected because the measured operation also contains:

- HashMap lookup
- BTreeMap lookup
- fixed function overhead
- cache behavior
- branch behavior
- memory effects
- benchmark noise

However, the experiment clearly demonstrates that increasing the number of orders at the target price increases the cancellation cost.

This is consistent with the expected:

```text
O(K)
```

behavior of:

```rust
queue.iter().position(...)
```

---

# Summary of Cancellation Complexity

The current implementation can therefore be represented as:

```text
                  cancel_order()
                       │
                       ▼
               HashMap lookup
                  O(1) avg
                       │
                       ▼
              BTreeMap lookup
                  O(log P)
                       │
                       ▼
              VecDeque search
                   O(K)
                       │
                       ▼
                  Remove
```

Overall:

```text
O(log P + K)
```

---

# Old Vec vs Current HashMap Index

| Property | Vec-Based Book | BTreeMap + HashMap |
|---|---|---|
| Price ordering | No | Yes |
| Best bid | O(N) | O(1) |
| Best ask | O(N) | O(1) |
| Can match | O(N) | O(1) |
| Order-ID lookup | O(N) | O(1) average |
| Cancel order | O(N) | O(log P + K) |
| FIFO at same price | Not explicitly represented | VecDeque |
| Price-time priority | More difficult | Natural representation |

The HashMap migration specifically changes the order-ID lookup path.

The BTreeMap migration is responsible for the price-ordering improvements.

Together, they provide a much more appropriate representation for a price-time-priority order book.

---

# Add Order Complexity

The current `add_order()` path also benefits from the HashMap.

Previously, duplicate-ID detection required scanning the order book.

That made the complete operation approximately:

```text
O(N)
```

The new duplicate check is:

```rust
self.order_index.contains_key(&order.id)
```

Average:

```text
O(1)
```

The order is then inserted into a BTreeMap price level.

Therefore the insertion path is approximately:

```text
duplicate check    O(1) average
BTreeMap insertion O(log P)
queue insertion    O(1) amortized
HashMap insertion  O(1) average
```

Overall:

```text
O(log P)
```

under the normal average-case HashMap assumptions.

---

# Index Consistency

The HashMap is now a secondary index.

That means it must remain synchronized with the actual order book.

## Adding an order

```text
OrderBook ← order
HashMap   ← order location
```

## Partial fill

```text
Order remains
Index remains
```

## Full fill

```text
Order removed
Index entry removed
```

## Cancellation

```text
Order removed
Index entry removed
```

The order-book test suite reached:

```text
26 / 26 tests passing
```

This is important because a stale index could cause incorrect duplicate-ID rejection or point to an invalid order location.

---

# Benchmarking Limitations

The exact timings should not be treated as universal performance numbers.

They depend on:

```text
CPU
Operating system
Rust compiler
Optimization settings
System load
Criterion configuration
Machine architecture
Cache state
```

Therefore, the main value of these benchmarks is:

```text
1. Comparing implementations
2. Observing scaling behavior
3. Validating expected algorithmic behavior
```

rather than treating an individual nanosecond value as universally meaningful.

---

# Gnuplot Message

Criterion reported:

```text
Gnuplot not found, using plotters backend
```

This is not a benchmark failure.

Criterion simply used its Plotters backend instead of Gnuplot.

The benchmarks executed successfully.

---

# What Changed Architecturally

The order book has evolved from a simple vector-based structure into a multi-index data structure.

## Initial design

```text
OrderBook

├── bids: Vec<Order>
└── asks: Vec<Order>
```

Main limitation:

```text
Search required for most operations
```

---

## BTreeMap migration

```text
OrderBook

├── bids: BTreeMap<Price, VecDeque<Order>>
└── asks: BTreeMap<Price, VecDeque<Order>>
```

Responsibilities:

```text
BTreeMap → price ordering
VecDeque → FIFO
```

This provides efficient price discovery.

---

## HashMap migration

```text
OrderBook

├── bids: BTreeMap<Price, VecDeque<Order>>
├── asks: BTreeMap<Price, VecDeque<Order>>
└── order_index: HashMap<u64, OrderLocation>
```

Responsibilities:

```text
BTreeMap → price ordering
VecDeque → FIFO
HashMap → order-ID lookup
```

This is a much closer representation of a real matching engine.

---

# Why This Design Is Useful for a Matching Engine

A matching engine does not have a single access pattern.

It needs to efficiently answer questions such as:

```text
What is the best bid?

What is the best ask?

Can this order match?

Which order has this ID?

Which orders are waiting at this price?

Which order gets priority?
```

Different operations benefit from different structures.

The current design therefore uses:

```text
                 OrderBook
                     │
        ┌────────────┼────────────┐
        │            │            │
        ▼            ▼            ▼
    BTreeMap      VecDeque      HashMap
        │            │            │
        ▼            ▼            ▼
    Price order     FIFO       Order lookup
```

This is an example of choosing data structures according to workload requirements.

---

# Remaining O(K) Optimization

The current cancellation implementation still searches the price-level queue:

```rust
queue.iter().position(...)
```

A future design could attempt to make order cancellation closer to direct lookup.

Conceptually:

```text
Current

Order ID
   ↓
HashMap
   ↓
Price
   ↓
search VecDeque
```

Potential future approach:

```text
Order ID
   ↓
HashMap
   ↓
direct order handle
```

However, this is not automatically the next optimization.

A direct handle introduces additional problems:

```text
Stable references
Queue mutation
FIFO preservation
Index maintenance
Memory layout
Cache locality
Removal semantics
```

In a high-performance matching engine, algorithmic complexity is important, but memory layout and cache behavior are also important.

Therefore the current `O(log P + K)` implementation is a reasonable intermediate design.

---

# Why We Should Not Optimize to O(1) Blindly

An `O(1)` theoretical operation is not automatically faster in practice.

The current `VecDeque` provides:

```text
Compact storage
FIFO ordering
Good iteration behavior
Simple ownership
```

Introducing another index could add:

```text
More memory
More writes
More bookkeeping
More cache pressure
More complexity
```

Therefore the correct approach is:

```text
Measure
   ↓
Identify bottleneck
   ↓
Change data structure
   ↓
Benchmark again
   ↓
Keep change only if it improves the real workload
```

This benchmark-driven approach is more appropriate for a systems project than optimizing solely from Big-O notation.

---

# Current OrderBook Complexity

| Operation | Current Structure | Complexity |
|---|---|---:|
| Add order | BTreeMap + HashMap | O(log P) |
| Best bid | BTreeMap | O(1) |
| Best ask | BTreeMap | O(1) |
| Can match | BTreeMap boundaries | O(1) |
| Order-ID lookup | HashMap | O(1) average |
| Cancel order | HashMap + BTreeMap + VecDeque | O(log P + K) |
| FIFO at price | VecDeque | O(1) front access/removal |

Where:

```text
P = number of price levels
K = orders at the target price
N = total orders in the book
```

---

# Current Architecture

```text
                    OrderBook
                       │
          ┌────────────┼────────────┐
          │            │            │
          ▼            ▼            ▼
        Bids          Asks       Order Index
          │            │            │
          ▼            ▼            ▼
      BTreeMap      BTreeMap      HashMap
          │            │            │
          ▼            ▼            ▼
      PriceLevel   PriceLevel   OrderLocation
          │            │
          ▼            ▼
      VecDeque      VecDeque
          │            │
          ▼            ▼
        Orders       Orders
```

This provides three important access patterns:

```text
Price → Orders
Price → Orders
Order ID → Location
```

---

# Milestone Status

The HashMap migration is now complete.

Completed:

```text
✓ OrderLocation introduced
✓ HashMap order index added
✓ Duplicate ID lookup migrated
✓ Index insertion implemented
✓ Full-fill index cleanup implemented
✓ Cancellation migrated to index
✓ Cancellation cleanup implemented
✓ Order ID reuse handled
✓ Tests updated
✓ 26/26 tests passing
✓ Cancellation benchmark completed
✓ Price-level O(K) benchmark completed
```

---

# Conclusion

The HashMap order index is a targeted optimization of the BTreeMap-based order book.

The previous cancellation path could require a whole-book search:

```text
Order ID
   ↓
Search entire book
   ↓
O(N)
```

The new path is:

```text
Order ID
   ↓
HashMap
   ↓
OrderLocation
   ↓
BTreeMap
   ↓
PriceLevel
   ↓
VecDeque search
   ↓
O(log P + K)
```

The dedicated price-level benchmark demonstrates the remaining `O(K)` component by increasing the number of orders at a single price from `1` to `10,000`.

The key architectural improvement is that the order book now has separate access structures for separate responsibilities:

```text
BTreeMap
    ↓
Price ordering

VecDeque
    ↓
FIFO / price-time priority

HashMap
    ↓
Order-ID lookup
```

This makes the data structure substantially closer to the requirements of a real matching engine.

---

# Next Experiment

The next major experiment should move from individual order-book operations to the actual matching path.

The target workflow is:

```text
Incoming Order
      ↓
Validation
      ↓
Best Bid / Best Ask
      ↓
Price-Time Matching
      ↓
Partial / Full Fills
      ↓
Trade Generation
      ↓
OrderBook Updates
      ↓
Process Result
```

The next benchmark should measure the complete matching workload rather than only:

```text
best_bid()
best_ask()
can_match()
cancel_order()
```

This will allow the project to transition from data-structure benchmarking toward measuring the performance of the actual matching engine.
