use serde::Serialize;
use tokio::sync::oneshot;
use crate::orderbook::OrderError;
use crate::{
    engine::ProcessResult,
    order::Order,
};

#[derive(Debug)]
pub enum OrderCommand {
    Add {
        order: Order,
        response: oneshot::Sender<OrderResponse>,
    },
}

#[derive(Debug, Serialize)]
pub enum OrderResponse {
    OrderProcessed(ProcessResult),
    OrderError(OrderError),

}