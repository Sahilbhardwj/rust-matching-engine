use crate::orderbook::OrderError;
use crate::{engine::ProcessResult, order::Order};
use serde::Serialize;
use tokio::sync::oneshot;

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
