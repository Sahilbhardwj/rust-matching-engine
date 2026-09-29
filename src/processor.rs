use std::println;

use tokio::sync::mpsc::Receiver;

use crate::{
    command::{OrderCommand, OrderResponse},
    engine::MatchingEngine,
    strategy::PriceTimeStrategy,
};

pub async fn run_orderbook(mut receiver: Receiver<OrderCommand>) {
    // The processor owns the matching engine.
    //         |
    //        And
    //         |
    //  The engine owns:
    // 1. OrderBook
    // 2. MatchingStrategy

    let mut engine = MatchingEngine::new(Box::new(PriceTimeStrategy));

    while let Some(command) = receiver.recv().await {
        match command {
            OrderCommand::Add { order, response } => {
                let result = engine.process_order(order);

                match result {
                    Ok(result) => {
                        let _ = response.send(OrderResponse::OrderProcessed(result));
                    }

                    Err(error) => {
                        println!("{}", error);
                        let _ = response.send(OrderResponse::OrderError(error));
                    }
                }
            }
        }
    }

    println!("OrderBook processor stopped");
}
