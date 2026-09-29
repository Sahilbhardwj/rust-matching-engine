use std::collections::{BTreeMap, VecDeque};
use crate::price::Price;
use crate::order::Order;
use crate::order::Side;
use crate::order::Trade;


pub type PriceLevel = VecDeque<Order>;   //at same price we can have multiple orders, so we use VecDeque to store them in FIFO order

#[derive(Debug)]
pub struct OrderBook {
    pub bids: BTreeMap<Price, PriceLevel>,
    pub asks: BTreeMap<Price, PriceLevel>
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
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

    if self.bids.values().flatten().any(|o| o.id == order.id)
        || self.asks.values().flatten().any(|o| o.id == order.id)
    {
        return Err(OrderError::DuplicateOrderId);
    }

    let price = Price::from_f64(order.price);

    match order.side {
        Side::Buy => {
            self.bids
                .entry(price)
                .or_default()
                .push_back(order);
        }

        Side::Sell => {
            self.asks
                .entry(price)
                .or_default()
                .push_back(order);
        }
    }

    Ok(())
}

   pub fn cancel_order(&mut self, order_id: u64) -> Option<Order> {
    for queue in self.bids.values_mut() {
        if let Some(index) = queue.iter().position(|order| order.id == order_id) {
            return queue.remove(index);
        }
    }

    for queue in self.asks.values_mut() {
        if let Some(index) = queue.iter().position(|order| order.id == order_id) {
            return queue.remove(index);
        }
    }

    None
}
 

   pub fn best_bid(&self) -> Option<Price> {
    self.bids
        .last_key_value()
        .map(|(price, _)| *price)
}

pub fn best_ask(&self) -> Option<Price> {
    self.asks
        .first_key_value()
        .map(|(price, _)| *price)
}


pub fn can_match(&self) -> bool {
    match (self.best_bid(), self.best_ask()) {
        (Some(bid_price), Some(ask_price)) => {
            bid_price >= ask_price
        }
        _ => false,
    }
}

pub fn execute_trade(
    &mut self,
    bid_price: Price,
    ask_price: Price,
) -> Option<Trade> {
    let bid_order = self.bids.get_mut(&bid_price)?.front_mut()?;
    let ask_order = self.asks.get_mut(&ask_price)?.front_mut()?;

    let trade_quantity = bid_order.quantity.min(ask_order.quantity);

    let trade_price = ask_order.price;

    let buy_order_id = bid_order.id;
    let sell_order_id = ask_order.id;

    bid_order.quantity -= trade_quantity;
    ask_order.quantity -= trade_quantity;

    let trade = Trade {
        buy_order_id,
        sell_order_id,
        price: trade_price,
        quantity: trade_quantity,
    };

    let bid_empty = self
        .bids
        .get(&bid_price)
        .map(|queue| queue.front().map_or(true, |order| order.quantity == 0.0))
        .unwrap_or(false);

    let ask_empty = self
        .asks
        .get(&ask_price)
        .map(|queue| queue.front().map_or(true, |order| order.quantity == 0.0))
        .unwrap_or(false);

    if bid_empty {
        if let Some(queue) = self.bids.get_mut(&bid_price) {
            queue.pop_front();
        }

        if self.bids.get(&bid_price).is_some_and(|queue| queue.is_empty()) {
            self.bids.remove(&bid_price);
        }
    }

    if ask_empty {
        if let Some(queue) = self.asks.get_mut(&ask_price) {
            queue.pop_front();
        }

        if self.asks.get(&ask_price).is_some_and(|queue| queue.is_empty()) {
            self.asks.remove(&ask_price);
        }
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



// unit tests for the OrderBook struct and its methods using arrangement, action, and assertion (AAA) pattern

#[cfg(test)]
mod tests {
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
        
       
       let mut book = OrderBook::new();
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

    
        let mut book = OrderBook::new();
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

    let mut book = OrderBook::new();
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

    let mut book = OrderBook::new();

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
    let mut book = OrderBook::new();
    //act
    let result = book.add_order(order);
    let result2=book.add_order(order2);
    //assert
    assert!(result.is_ok());
    assert_eq!(result2,Err(OrderError::DuplicateOrderId));
    assert_eq!(book.bids.len(),1);
}

    #[test]
fn test_cancel_order() {
  //arrange
     let order = Order {
        id: 1,
        price: 10.0,
        quantity: 10.0,
        side: Side::Buy,
    };
    
     let order2 = Order {
        id: 2,
        price: 10.0,
        quantity: 10.0,
        side: Side::Buy,
    };

    let orderid=order.id;
   let mut book = OrderBook::new();
    //act
    let result = book.add_order(order);
    let result2=book.add_order(order2);
    //assert
    assert!(result.is_ok());
    assert!(result2.is_ok());
    //act
    let final_result=book.cancel_order(orderid);
    //assert
    assert!(final_result.is_some());
    assert_eq!(final_result.unwrap().id, 1);
    assert_eq!(book.bids.len(),1);


}
    #[test]
fn test_cancel_nonexistent_order() {
   let mut book = OrderBook::new();
    let result=book.cancel_order(999);
    assert!(result.is_none());
}

#[test]
fn test_best_bid() {
    //arrange
     let order = Order {
        id: 1,
        price: 101.0,
        quantity: 10.0,
        side: Side::Buy,
    };
    
     let order2 = Order {
        id: 2,
        price: 102.0,
        quantity: 10.0,
        side: Side::Buy,
    };
    let order3 = Order {
        id: 3,
        price: 103.0,
        quantity: 10.0,
        side: Side::Buy,
    };

    let mut book = OrderBook::new();  
    
   let _ = book.add_order(order).unwrap();
    let _ = book.add_order(order2).unwrap();
    let _ = book.add_order(order3).unwrap();
   //act
    let best_bid_index=book.best_bid();
    //assert
    assert!(best_bid_index.is_some());
    let best_bid_price = book.best_bid().unwrap();   
    assert_eq!(best_bid_price, Price::from_f64(103.0));


}

    #[test]
fn test_best_ask() {
    //arrange
     let order = Order {
        id: 1,
        price: 101.0,
        quantity: 10.0,
        side: Side::Sell,
    };
    
     let order2 = Order {
        id: 2,
        price: 102.0,
        quantity: 10.0,
        side: Side::Sell,
    };
    let order3 = Order {
        id: 3,
        price: 103.0,
        quantity: 10.0,
        side: Side::Sell,
    };

    let mut book = OrderBook::new(); 
    
   let _ = book.add_order(order).unwrap();
    let _ = book.add_order(order2).unwrap();
    let _ = book.add_order(order3).unwrap();
   //act
   let best_ask_price = book.best_ask().unwrap();

   assert_eq!(best_ask_price, Price::from_f64(101.0));


}
#[test]
fn test_empty_orderbook() {
    // create empty OrderBook
   let book = OrderBook::new();
    assert!(book.best_ask().is_none());
    assert!(book.best_bid().is_none());
    assert!(!book.can_match());

}
#[test]
fn test_can_match() {
    let mut book = OrderBook::new();
    let _ = book.add_order(Order {
        id: 1,
        price: 100.0,
        quantity: 10.0,
        side: Side::Buy,
    }).unwrap();
    let _ = book.add_order(Order {
        id: 2,
        price: 90.0,
        quantity: 10.0,
        side: Side::Sell,
    }).unwrap();
    assert!(book.can_match());

}

#[test]
fn test_cannot_match() {
    let mut book = OrderBook::new();
    let _ = book.add_order(Order {
        id: 1,
        price: 80.0,
        quantity: 10.0,
        side: Side::Buy,
    }).unwrap();
    let _ = book.add_order(Order {
        id: 2,
        price: 90.0,
        quantity: 10.0,
        side: Side::Sell,
    }).unwrap();
    assert!(!book.can_match());
}
#[test]
fn test_execute_trade_full_fill() {
   
    let mut book = OrderBook::new();
     book.add_order(Order {
        id: 1,
        price: 100.0,
        quantity: 10.0,
        side: Side::Buy,
    }).unwrap();
    book.add_order(Order {
        id: 2,
        price: 90.0,
        quantity: 10.0,
        side: Side::Sell,
    }).unwrap();

    // getting the best_bid and best_ask indices
    let bid_index = book.best_bid().unwrap();
    let ask_index = book.best_ask().unwrap();
    // Act
    let trade = book.execute_trade(bid_index, ask_index).unwrap();
    

    // Assert
    assert_eq!(trade.quantity, 10.0);
    assert_eq!(trade.price, 90.0);
    assert_eq!(trade.buy_order_id, 1);
    assert_eq!(trade.sell_order_id, 2);
    assert!(book.bids.is_empty());
    assert!(book.asks.is_empty());
    
}
#[test]
fn test_execute_trade_partial_fill() {
    let mut book = OrderBook::new();

    // Buy order has greater quantity than sell order
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
        quantity: 5.0,
        side: Side::Sell,
    })
    .unwrap();

    // Get best prices
    let bid_price = book.best_bid().unwrap();
    let ask_price = book.best_ask().unwrap();

    // Act
    let trade = book.execute_trade(bid_price, ask_price).unwrap();

    // Assert
    assert_eq!(trade.quantity, 5.0);
    assert_eq!(trade.price, 90.0);
    assert_eq!(trade.buy_order_id, 1);
    assert_eq!(trade.sell_order_id, 2);

    // Buy order should have 5 remaining
    let remaining_bid = book
        .bids
        .get(&bid_price)
        .unwrap()
        .front()
        .unwrap();

    assert_eq!(remaining_bid.quantity, 5.0);

    // Sell order was completely filled
    assert!(book.asks.is_empty());


    // --------------------------------------------------
    // Second partial fill:
    // Remaining buy = 5
    // New sell = 10
    // --------------------------------------------------

    book.add_order(Order {
        id: 2,
        price: 90.0,
        quantity: 10.0,
        side: Side::Sell,
    })
    .unwrap();

    let bid_price = book.best_bid().unwrap();
    let ask_price = book.best_ask().unwrap();

    // Act
    let trade = book.execute_trade(bid_price, ask_price).unwrap();

    // Assert
    assert_eq!(trade.quantity, 5.0);
    assert_eq!(trade.price, 90.0);
    assert_eq!(trade.buy_order_id, 1);
    assert_eq!(trade.sell_order_id, 2);

    // Buy order was completely filled
    assert!(book.bids.is_empty());

    // Sell order has 5 remaining
    let remaining_ask = book
        .asks
        .get(&ask_price)
        .unwrap()
        .front()
        .unwrap();

    assert_eq!(remaining_ask.quantity, 5.0);
}        
}

