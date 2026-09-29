use crate::order::Trade;
use crate::orderbook::OrderBook;

pub trait MatchingStrategy: Send {
    fn match_orders(&self, book: &mut OrderBook) -> Vec<Trade>;
}

pub struct PriceTimeStrategy;

impl MatchingStrategy for PriceTimeStrategy {
    fn match_orders(&self, book: &mut OrderBook) -> Vec<Trade> {
        let mut trades = Vec::new();

        while book.can_match() {
            let bid_price = match book.best_bid() {
                Some(price) => price,
                None => break,
            };

            let ask_price = match book.best_ask() {
                Some(price) => price,
                None => break,
            };

            if let Some(trade) = book.execute_trade(bid_price, ask_price) {
                trades.push(trade);
            } else {
                break;
            }
        }

        trades
    }
}

//unit tests for strategy.rs


#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::{Order, Side};

    #[test]
    fn test_no_match() {
        let mut book = OrderBook::new();

        book.add_order(Order {
            id: 1,
            price: 100.0,
            quantity: 10.0,
            side: Side::Buy,
        })
        .unwrap();

        book.add_order(Order {
            id: 2,
            price: 110.0,
            quantity: 10.0,
            side: Side::Sell,
        })
        .unwrap();

        let strategy = PriceTimeStrategy;

        let trades = strategy.match_orders(&mut book);

        assert!(trades.is_empty());

        assert_eq!(book.bids.len(), 1);
        assert_eq!(book.asks.len(), 1);
    }

    #[test]
    fn test_single_match() {
        let mut book = OrderBook::new();

        book.add_order(Order {
            id: 1,
            price: 100.0,
            quantity: 10.0,
            side: Side::Buy,
        })
        .unwrap();

        book.add_order(Order {
            id: 2,
            price: 90.0,
            quantity: 10.0,
            side: Side::Sell,
        })
        .unwrap();

        let strategy = PriceTimeStrategy;

        let trades = strategy.match_orders(&mut book);

        assert_eq!(trades.len(), 1);

        let trade = &trades[0];

        assert_eq!(trade.buy_order_id, 1);
        assert_eq!(trade.sell_order_id, 2);
        assert_eq!(trade.price, 90.0);
        assert_eq!(trade.quantity, 10.0);

        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
    }

    #[test]
    fn test_best_price_priority() {
        let mut book = OrderBook::new();

        // More expensive sell order added first
        book.add_order(Order {
            id: 2,
            price: 100.0,
            quantity: 5.0,
            side: Side::Sell,
        })
        .unwrap();

        // Cheaper sell order added second
        book.add_order(Order {
            id: 3,
            price: 90.0,
            quantity: 5.0,
            side: Side::Sell,
        })
        .unwrap();

        // Incoming buy can match both
        book.add_order(Order {
            id: 1,
            price: 110.0,
            quantity: 10.0,
            side: Side::Buy,
        })
        .unwrap();

        let strategy = PriceTimeStrategy;

        let trades = strategy.match_orders(&mut book);

        assert_eq!(trades.len(), 2);

        // Cheaper sell order must execute first
        assert_eq!(trades[0].sell_order_id, 3);
        assert_eq!(trades[0].price, 90.0);
        assert_eq!(trades[0].quantity, 5.0);

        // More expensive sell order executes next
        assert_eq!(trades[1].sell_order_id, 2);
        assert_eq!(trades[1].price, 100.0);
        assert_eq!(trades[1].quantity, 5.0);
    }

    #[test]
    fn test_partial_fill() {
        let mut book = OrderBook::new();

        book.add_order(Order {
            id: 2,
            price: 90.0,
            quantity: 5.0,
            side: Side::Sell,
        })
        .unwrap();

        book.add_order(Order {
            id: 1,
            price: 100.0,
            quantity: 10.0,
            side: Side::Buy,
        })
        .unwrap();

        let strategy = PriceTimeStrategy;

        let trades = strategy.match_orders(&mut book);

        assert_eq!(trades.len(), 1);

        assert_eq!(trades[0].quantity, 5.0);
        assert_eq!(trades[0].price, 90.0);

        // Sell completely filled
        assert!(book.asks.is_empty());

        // Buy has 5 remaining
        assert_eq!(book.bids.len(), 1);

        let bid_price = book.best_bid().unwrap();

        let remaining_bid = book
            .bids
            .get(&bid_price)
            .unwrap()
            .front()
            .unwrap();

        assert_eq!(remaining_bid.id, 1);
        assert_eq!(remaining_bid.quantity, 5.0);
    }

    #[test]
    fn test_time_priority_same_price() {
        let mut book = OrderBook::new();

        // Arrived first
        book.add_order(Order {
            id: 2,
            price: 90.0,
            quantity: 5.0,
            side: Side::Sell,
        })
        .unwrap();

        // Arrived second
        book.add_order(Order {
            id: 3,
            price: 90.0,
            quantity: 5.0,
            side: Side::Sell,
        })
        .unwrap();

        book.add_order(Order {
            id: 1,
            price: 100.0,
            quantity: 5.0,
            side: Side::Buy,
        })
        .unwrap();

        let strategy = PriceTimeStrategy;

        let trades = strategy.match_orders(&mut book);

        assert_eq!(trades.len(), 1);

        // Earlier order at the same price gets filled first
        assert_eq!(trades[0].sell_order_id, 2);
        assert_eq!(trades[0].quantity, 5.0);

        // Later order remains
        assert_eq!(book.asks.len(), 1);

        let ask_price = book.best_ask().unwrap();

        let remaining_ask = book
            .asks
            .get(&ask_price)
            .unwrap()
            .front()
            .unwrap();

        assert_eq!(remaining_ask.id, 3);
    }
}