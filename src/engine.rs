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