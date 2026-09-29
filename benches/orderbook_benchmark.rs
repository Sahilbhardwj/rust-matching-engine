use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

use orderbook_modules::{
    order::{Order, Side},
    orderbook::OrderBook,
};

fn benchmark_best_bid(c: &mut Criterion) {
    for size in [1_000, 10_000, 100_000] {
        let mut book = OrderBook::new();

        for id in 1..=size {
            book.add_order(Order {
                id,
                price: id as f64,
                quantity: 1.0,
                side: Side::Buy,
            })
            .unwrap();
        }

        c.bench_function(&format!("best_bid_{}", size), |b| {
            b.iter(|| {
                black_box(book.best_bid());
            });
        });
    }
}

fn benchmark_best_ask(c: &mut Criterion) {
    for size in [1_000, 10_000, 100_000] {
        let mut book = OrderBook::new();

        for id in 1..=size {
            book.add_order(Order {
                id,
                price: id as f64,
                quantity: 1.0,
                side: Side::Sell,
            })
            .unwrap();
        }

        c.bench_function(&format!("best_ask_{}", size), |b| {
            b.iter(|| {
                black_box(book.best_ask());
            });
        });
    }
}

fn benchmark_can_match(c: &mut Criterion) {
    for size in [1_000, 10_000, 100_000] {
        let mut book = OrderBook::new();

        // Buy orders
        for id in 1..=size {
            book.add_order(Order {
                id,
                price: 100.0 + id as f64,
                quantity: 1.0,
                side: Side::Buy,
            })
            .unwrap();
        }

        // Sell orders
        for id in 1..=size {
            book.add_order(Order {
                id: size + id,
                price: 1.0 + id as f64,
                quantity: 1.0,
                side: Side::Sell,
            })
            .unwrap();
        }

        c.bench_function(&format!("can_match_{}", size), |b| {
            b.iter(|| {
                black_box(book.can_match());
            });
        });
    }
}

fn benchmark_cancel_order(c: &mut Criterion) {
    for size in [1_000, 10_000, 100_000] {
        c.bench_function(&format!("cancel_order_{}", size), |b| {
            b.iter_batched(
                || {
                    let mut book = OrderBook::new();

                    for id in 1..=size {
                        book.add_order(Order {
                            id,
                            price: id as f64,
                            quantity: 1.0,
                            side: Side::Buy,
                        })
                        .unwrap();
                    }

                    book
                },
                |mut book| {
                    // Cancel the last order.
                    //
                    // Vec implementation:
                    // requires scanning the book to find the order.
                    //
                    // HashMap-index implementation:
                    // finds the order's price level directly,
                    // then searches only that price level.
                    black_box(book.cancel_order(size));
                },
                BatchSize::SmallInput,
            );
        });
    }
}

criterion_group!(
    benches,
    benchmark_best_bid,
    benchmark_best_ask,
    benchmark_can_match,
    benchmark_cancel_order
);

criterion_main!(benches);
