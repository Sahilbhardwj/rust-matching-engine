use serde::Serialize;
use crate::order::{Order, Trade};
use crate::orderbook::{OrderBook, OrderError};
use crate::strategy::MatchingStrategy;

#[derive(Debug,Serialize)]
pub struct ProcessResult {
    pub order_id: u64,
    pub trades: Vec<Trade>,
}

pub struct MatchingEngine {
    pub book: OrderBook,
    pub strategy: Box<dyn MatchingStrategy>,
}

impl MatchingEngine {
    pub fn new(strategy: Box<dyn MatchingStrategy>) -> Self {
        Self {
            book: OrderBook::new(),
            strategy,
        }
    }

    pub fn process_order(&mut self, order: Order) -> Result<ProcessResult, OrderError>{
        let order_id = order.id;

        // Add incoming order to the book
        self.book.add_order(order)?;

        // Let the selected strategy perform matching
        let trades = self.strategy.match_orders(&mut self.book);

       Ok(ProcessResult {
            order_id,
            trades,
        }) 
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

        let remaining_bid = engine
            .book
            .bids
            .get(&bid_price)
            .unwrap()
            .front()
            .unwrap();

        assert_eq!(remaining_bid.id, 1);
        assert_eq!(remaining_bid.quantity, 5.0);
    }

    #[test]
    fn test_process_order_multiple_trades() {
        let strategy = Box::new(PriceTimeStrategy);
        let mut engine = MatchingEngine::new(strategy);

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

        let remaining_bid = engine
            .book
            .bids
            .get(&bid_price)
            .unwrap()
            .front()
            .unwrap();

        assert_eq!(remaining_bid.id, 1);
        assert_eq!(remaining_bid.quantity, 3.0);

        assert!(engine.book.asks.is_empty());
    }

    #[test]
    fn test_process_order_invalid_orders() {
        let strategy = Box::new(PriceTimeStrategy);
        let mut engine = MatchingEngine::new(strategy);

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

        let remaining_bid = engine
            .book
            .bids
            .get(&bid_price)
            .unwrap()
            .front()
            .unwrap();

        assert_eq!(remaining_bid.id, 3);
    }
}