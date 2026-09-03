

use crate::order::Order;
use crate::order::Side;
use crate::order::Trade;
#[derive(Debug)]
pub struct OrderBook {
    pub bids: Vec<Order>,
    pub asks:Vec<Order>
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: Vec::new(),
            asks:Vec::new(),
        }
    }

    pub fn add_order(
    &mut self,
    order: Order,
    ) -> Result<(), OrderError> {

    if order.price <= 0.0 {
        return Err(OrderError::InvalidPrice);
    }

    if order.quantity <= 0.0 {
        return Err(OrderError::InvalidQuantity);
    }

    if self.bids.iter().any(|o| o.id == order.id)
        || self.asks.iter().any(|o| o.id == order.id)
    {
        return Err(OrderError::DuplicateOrderId);
    }

    match order.side {
        Side::Buy => self.bids.push(order),
        Side::Sell => self.asks.push(order),
    }

    Ok(())
}

    pub fn cancel_order(&mut self, order_id: u64) -> Option<Order> {
    // First search bids
    if let Some(index) = self
        .bids
        .iter()
        .position(|order| order.id == order_id)
    {
        return Some(self.bids.remove(index));
    }

    // If not found, search asks
    if let Some(index) = self
        .asks
        .iter()
        .position(|order| order.id == order_id)
    {
        return Some(self.asks.remove(index));
    }

    // Order doesn't exist
    None
}
 

    pub fn best_bid(&self) -> Option<usize> {
    self.bids
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.price.total_cmp(&b.price))
        .map(|(index, _)| index)
}

pub fn best_ask(&self) -> Option<usize> {
    self.asks
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.price.total_cmp(&b.price))
        .map(|(index, _)| index)
}


pub fn can_match(&self) -> bool {
    match (self.best_bid(), self.best_ask()) {
        (Some(bid_index), Some(ask_index)) => {
            self.bids[bid_index].price >= self.asks[ask_index].price
        }
        _ => false,
    }
}

pub fn execute_trade( &mut self,  bid_index: usize, ask_index: usize,) -> Option<Trade> {
    let trade_quantity = self.bids[bid_index]
        .quantity
        .min(self.asks[ask_index].quantity);

    let trade_price = self.asks[ask_index].price;

    let buy_order_id = self.bids[bid_index].id;
    let sell_order_id = self.asks[ask_index].id;

    // Update quantities
    self.bids[bid_index].quantity -= trade_quantity;
    self.asks[ask_index].quantity -= trade_quantity;

    let trade = Trade {
        buy_order_id,
        sell_order_id,
        price: trade_price,
        quantity: trade_quantity,
    };

    // Remove completely filled orders
    if self.bids[bid_index].quantity == 0.0 {
        self.bids.remove(bid_index);
    }

    if self.asks[ask_index].quantity == 0.0 {
        self.asks.remove(ask_index);
    }

    Some(trade)
}
        
        }




#[derive(Debug, thiserror::Error, serde::Serialize,PartialEq)]
pub enum OrderError {
    #[error("Order ID already exists")]
    DuplicateOrderId,

    #[error("Order price must be greater than zero")]
    InvalidPrice,

    #[error("Order quantity must be greater than zero")]
    InvalidQuantity,
}


#[cfg(test)]
mod tests{
    use std::{assert_eq};

use super::*;

    #[test]
    fn test_buy_add_order() {
        // A
        let buy_order=Order{
            id:1,
            price:100.0,
            quantity:10.0,
            side:Side::Buy,
        };
        
       
        let  mut book=OrderBook{
            bids:Vec::new(),
            asks:Vec::new(),
        };
    //A
     let result=book.add_order(buy_order);

    //A
     assert!(result.is_ok());
     assert_eq!(book.bids.len(),1);

    

    }
    
    #[test]
   fn test_sell_add_order(){
    //arrange
    let sell_order=Order{
            id:2,
            price:100.0,
            quantity:10.0,
            side:Side::Sell,
        };

    
        let  mut book=OrderBook{
            bids:Vec::new(),
            asks:Vec::new(),
        };
     
     // act
    let result=book.add_order(sell_order);
    //assert
     assert!(result.is_ok());
     assert_eq!(book.asks.len(),1);
   }

   #[test]
fn test_invalid_price() {
    let order = Order {
        id: 1,
        price: 0.0,
        quantity: 10.0,
        side: Side::Buy,
    };

    let mut book = OrderBook {
        bids: Vec::new(),
        asks: Vec::new(),
    };

    let result = book.add_order(order);

    assert_eq!(result, Err(OrderError::InvalidPrice));
    assert_eq!(book.bids.len(), 0);
}
#[test]
fn test_invalid_quantity() {
    let order = Order {
        id: 1,
        price: 100.0,
        quantity: 0.0,
        side: Side::Buy,
    };

    let mut book = OrderBook {
        bids: Vec::new(),
        asks: Vec::new(),
    };

    let result = book.add_order(order);

    assert_eq!(result, Err(OrderError::InvalidQuantity));
    assert_eq!(book.bids.len(), 0);
}

#[test]
fn test_duplicate_order_id_is_rejected() {
 // arrange
     let order = Order {
        id: 1,
        price: 10.0,
        quantity: 10.0,
        side: Side::Buy,
    };
    
     let order2 = Order {
        id: 1,
        price: 10.0,
        quantity: 10.0,
        side: Side::Buy,
    };
    let mut book = OrderBook {
        bids: Vec::new(),
        asks: Vec::new(),
    };
    //act
    let result = book.add_order(order);
    let result2=book.add_order(order2);
    //assert
    assert!(result.is_ok());
    assert_eq!(result2,Err(OrderError::DuplicateOrderId));
    assert_eq!(book.bids.len(),1);
    
    

}
}
