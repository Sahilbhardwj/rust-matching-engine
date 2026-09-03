use serde::{Deserialize, Serialize};


#[derive(Debug,Deserialize,Serialize)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug,Deserialize,Serialize)]
pub struct Order {
    pub id: u64,
    pub price: f64,
    pub quantity: f64,
    pub side: Side,
}

#[derive(Debug,Deserialize,Serialize)]
pub struct Trade {
    pub buy_order_id: u64,
    pub sell_order_id: u64,
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug,Deserialize,Serialize)]
pub struct ProcessResult {
    pub order_id: u64,
    pub trades: Vec<Trade>,
}