# Benchmark Results

This directory contains performance benchmarks for the Rust matching engine.

The benchmarks are used to evaluate order-book data structures and measure
the impact of architectural and data-structure changes.

## Benchmark tool

Benchmarks are written using [Criterion](https://github.com/bheisler/criterion.rs)
and executed using Cargo's release benchmark profile.

## Methodology

Benchmarks measure individual order-book operations rather than including
order-book construction inside the measured operation.

Multiple order-book sizes are tested to observe how performance scales with
the number of orders.

Current benchmark sizes:

- 1,000 orders
- 10,000 orders
- 100,000 orders

## Benchmark progression

The performance analysis follows this process:

1. Establish a baseline using `Vec<Order>`.
2. Identify performance characteristics and bottlenecks.
3. Introduce a price-ordered data structure.
4. Run the same workloads against the new implementation.
5. Compare the results.
6. Analyze the trade-offs of the new design.

## Current baseline

See [`baseline-vec.md`](./baseline-vec.md).

## Future comparisons

Additional benchmark results will be added as the order-book data structure
is optimized.