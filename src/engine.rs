use crate::engine_state::EngineLifecycle;
use crate::metrics::EngineMetrics;
use crate::order::{Order, Trade};
use crate::orderbook::{OrderBook, OrderError};
use crate::strategy::MatchingStrategy;
use serde::Serialize;
use std::sync::{Arc,atomic::Ordering};

#[derive(Debug, Serialize)]
pub struct ProcessResult {
    pub order_id: u64,
    pub trades: Vec<Trade>,
}

pub struct MatchingEngine {
    pub book: OrderBook,
    pub strategy: Box<dyn MatchingStrategy>,
    metrics: Arc<EngineMetrics>,
    lifecycle: EngineLifecycle,
}

impl MatchingEngine {
    pub fn new(strategy: Box<dyn MatchingStrategy>) -> Self {
        Self {
            book: OrderBook::new(),
            strategy,
            metrics: Arc::new(EngineMetrics::new()),
            lifecycle: EngineLifecycle::new(),
        }
    }

    pub fn process_order(&mut self, order: Order) -> Result<ProcessResult, OrderError> {
     //ensure the engine is running before processing orders
        if !self.lifecycle.is_running() {
       return Err(OrderError::EngineNotRunning);
       }
        let order_id = order.id;

        // Add incoming order to the book
        self.book.add_order(order)?;
        self.metrics.orders_processed.fetch_add(1, Ordering::Relaxed);

        // Let the selected strategy perform matching
        let trades = self.strategy.match_orders(&mut self.book);

        self.metrics.trades_executed.fetch_add(trades.len() as u64, Ordering::Relaxed);
        Ok(ProcessResult { order_id, trades })
    }

    pub fn metrics(&self) -> Arc<EngineMetrics> {
    Arc::clone(&self.metrics)
    }

    pub fn start(&self) -> bool {
        self.lifecycle.start()
    }

    pub fn shutdown(&self) -> bool {
    self.lifecycle.shutdown()
}
}



//unit tests for MatchingEngine
#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::Side;
    use crate::strategy::PriceTimeStrategy;

    #[test]
    fn test_process_order_without_match() {
        let strategy = Box::new(PriceTimeStrategy);
        let mut engine = MatchingEngine::new(strategy);
        assert!(engine.start());
        let order = Order {
            id: 1,
            price: 100.0,
            quantity: 10.0,
            side: Side::Buy,
        };

        let result = engine.process_order(order).unwrap();

        assert_eq!(result.order_id, 1);
        assert!(result.trades.is_empty());

        assert_eq!(engine.book.bids.len(), 1);
        assert!(engine.book.asks.is_empty());
    }

    #[test]
    fn test_process_order_with_match() {
        let strategy = Box::new(PriceTimeStrategy);
        let mut engine = MatchingEngine::new(strategy);
        assert!(engine.start());
        let buy_order = Order {
            id: 1,
            price: 100.0,
            quantity: 10.0,
            side: Side::Buy,
        };

        engine.process_order(buy_order).unwrap();

        let sell_order = Order {
            id: 2,
            price: 90.0,
            quantity: 10.0,
            side: Side::Sell,
        };

        let result = engine.process_order(sell_order).unwrap();

        assert_eq!(result.order_id, 2);
        assert_eq!(result.trades.len(), 1);

        let trade = &result.trades[0];

        assert_eq!(trade.buy_order_id, 1);
        assert_eq!(trade.sell_order_id, 2);
        assert_eq!(trade.price, 90.0);
        assert_eq!(trade.quantity, 10.0);

        assert!(engine.book.bids.is_empty());
        assert!(engine.book.asks.is_empty());
    }

    #[test]
    fn test_process_order_partial_fill() {
        let strategy = Box::new(PriceTimeStrategy);
        let mut engine = MatchingEngine::new(strategy);
        assert!(engine.start());
        let sell_order = Order {
            id: 2,
            price: 90.0,
            quantity: 5.0,
            side: Side::Sell,
        };

        engine.process_order(sell_order).unwrap();

        let buy_order = Order {
            id: 1,
            price: 100.0,
            quantity: 10.0,
            side: Side::Buy,
        };

        let result = engine.process_order(buy_order).unwrap();

        assert_eq!(result.order_id, 1);
        assert_eq!(result.trades.len(), 1);

        let trade = &result.trades[0];

        assert_eq!(trade.buy_order_id, 1);
        assert_eq!(trade.sell_order_id, 2);
        assert_eq!(trade.price, 90.0);
        assert_eq!(trade.quantity, 5.0);

        assert!(engine.book.asks.is_empty());
        assert_eq!(engine.book.bids.len(), 1);

        // Get the remaining order from the best bid price level
        let bid_price = engine.book.best_bid().unwrap();

        let remaining_bid = engine.book.bids.get(&bid_price).unwrap().front().unwrap();

        assert_eq!(remaining_bid.id, 1);
        assert_eq!(remaining_bid.quantity, 5.0);
    }

    #[test]
    fn test_process_order_multiple_trades() {
        let strategy = Box::new(PriceTimeStrategy);
        let mut engine = MatchingEngine::new(strategy);
        assert!(engine.start());
        // Existing sell orders
        engine
            .process_order(Order {
                id: 2,
                price: 90.0,
                quantity: 3.0,
                side: Side::Sell,
            })
            .unwrap();

        engine
            .process_order(Order {
                id: 3,
                price: 95.0,
                quantity: 4.0,
                side: Side::Sell,
            })
            .unwrap();

        // Incoming buy order
        let result = engine
            .process_order(Order {
                id: 1,
                price: 100.0,
                quantity: 10.0,
                side: Side::Buy,
            })
            .unwrap();

        assert_eq!(result.order_id, 1);
        assert_eq!(result.trades.len(), 2);

        // First trade
        assert_eq!(result.trades[0].sell_order_id, 2);
        assert_eq!(result.trades[0].price, 90.0);
        assert_eq!(result.trades[0].quantity, 3.0);

        // Second trade
        assert_eq!(result.trades[1].sell_order_id, 3);
        assert_eq!(result.trades[1].price, 95.0);
        assert_eq!(result.trades[1].quantity, 4.0);

        // 10 requested - 7 filled = 3 remaining
        assert_eq!(engine.book.bids.len(), 1);

        let bid_price = engine.book.best_bid().unwrap();

        let remaining_bid = engine.book.bids.get(&bid_price).unwrap().front().unwrap();

        assert_eq!(remaining_bid.id, 1);
        assert_eq!(remaining_bid.quantity, 3.0);

        assert!(engine.book.asks.is_empty());
    }

    #[test]
    fn test_process_order_invalid_orders() {
        let strategy = Box::new(PriceTimeStrategy);
        let mut engine = MatchingEngine::new(strategy);
        assert!(engine.start());
        // Invalid price
        let result = engine.process_order(Order {
            id: 1,
            price: 0.0,
            quantity: 10.0,
            side: Side::Buy,
        });

        assert!(matches!(result, Err(OrderError::InvalidPrice)));

        // Invalid quantity
        let result = engine.process_order(Order {
            id: 2,
            price: 100.0,
            quantity: 0.0,
            side: Side::Buy,
        });

        assert!(matches!(result, Err(OrderError::InvalidQuantity)));

        // Add a valid order first
        engine
            .process_order(Order {
                id: 3,
                price: 100.0,
                quantity: 10.0,
                side: Side::Buy,
            })
            .unwrap();

        // Duplicate ID
        let result = engine.process_order(Order {
            id: 3,
            price: 110.0,
            quantity: 5.0,
            side: Side::Buy,
        });

        assert!(matches!(result, Err(OrderError::DuplicateOrderId)));

        // Only the valid order should remain
        assert_eq!(engine.book.bids.len(), 1);

        let bid_price = engine.book.best_bid().unwrap();

        let remaining_bid = engine.book.bids.get(&bid_price).unwrap().front().unwrap();

        assert_eq!(remaining_bid.id, 3);
    }

    #[test]
fn test_engine_metrics() {
    let strategy = Box::new(PriceTimeStrategy);
    let mut engine = MatchingEngine::new(strategy);
    assert!(engine.start());
    engine
        .process_order(Order {
            id: 1,
            price: 100.0,
            quantity: 10.0,
            side: Side::Buy,
        })
        .unwrap();

    engine
        .process_order(Order {
            id: 2,
            price: 90.0,
            quantity: 10.0,
            side: Side::Sell,
        })
        .unwrap();

    assert_eq!(engine.metrics.orders_processed.load(Ordering::Relaxed), 2);
    assert_eq!(engine.metrics.trades_executed.load(Ordering::Relaxed), 1);
}

#[test]
fn test_metrics_can_be_read_concurrently() {
    let strategy = Box::new(PriceTimeStrategy);
    let mut engine = MatchingEngine::new(strategy);
    assert!(engine.start());
    let metrics = engine.metrics();
    let reporter_metrics = Arc::clone(&metrics);

    let reporter = std::thread::spawn(move || {
        for _ in 0..1000 {
            let _orders = reporter_metrics
                .orders_processed
                .load(Ordering::Relaxed);

            let _trades = reporter_metrics
                .trades_executed
                .load(Ordering::Relaxed);

            std::thread::yield_now();
        }
    });

    for id in 1..=1000 {
        engine
            .process_order(Order {
                id,
                price: 100.0,
                quantity: 1.0,
                side: Side::Buy,
            })
            .unwrap();
    }

    reporter.join().unwrap();

    assert_eq!(metrics.orders_processed.load(Ordering::Relaxed), 1000);
}

 #[test]
    fn test_engine_can_start_once() {
        let strategy = Box::new(PriceTimeStrategy);
        let engine = MatchingEngine::new(strategy);

        // First start: Stopped -> Running
        assert!(engine.start());

        // Second start: already Running, so it must fail
        assert!(!engine.start());
    }
    #[test]
fn test_engine_shutdown() {
    let strategy = Box::new(PriceTimeStrategy);
    let engine = MatchingEngine::new(strategy);

    // Cannot shut down before starting
    assert!(!engine.shutdown());

    // Start engine
    assert!(engine.start());

    // Running -> ShuttingDown
    assert!(engine.shutdown());

    // Cannot shut down twice
    assert!(!engine.shutdown());
}
#[test]
fn test_engine_rejects_order_when_not_running() {
    let strategy = Box::new(PriceTimeStrategy);
    let mut engine = MatchingEngine::new(strategy);

    let order = Order {
        id: 1,
        price: 100.0,
        quantity: 10.0,
        side: Side::Buy,
    };

    let result = engine.process_order(order);

    assert!(matches!(result, Err(OrderError::EngineNotRunning)));
}
}
