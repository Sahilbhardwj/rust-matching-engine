use crate::order::{Trade};
use crate::orderbook::OrderBook;

pub trait MatchingStrategy:Send {
    fn match_orders(&self, book: &mut OrderBook) -> Vec<Trade>;
}

pub struct PriceTimeStrategy;

impl MatchingStrategy for PriceTimeStrategy {
    fn match_orders(&self, book: &mut OrderBook) -> Vec<Trade> {
        let mut trades = Vec::new();

        while book.can_match() {
            let bid_index = match book.best_bid() {
                Some(index) => index,
                None => break,
            };

            let ask_index = match book.best_ask() {
                Some(index) => index,
                None => break,
            };

            if let Some(trade) = book.execute_trade(bid_index, ask_index) {
                trades.push(trade);
            } else {
                break;
            }
        }

        trades
    }
}