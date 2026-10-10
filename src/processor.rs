use std::println;

use tokio::sync::{
    mpsc::Receiver,
    oneshot::Receiver as ShutdownReceiver,
};

use crate::{
    command::{OrderCommand, OrderResponse},
    engine::MatchingEngine,
    strategy::PriceTimeStrategy,
};

pub async fn run_orderbook(
    mut receiver: Receiver<OrderCommand>,
    mut shutdown: ShutdownReceiver<()>,
) {
    // The processor owns the matching engine.
    //
    // The engine owns:
    // 1. OrderBook
    // 2. MatchingStrategy

    let mut engine = MatchingEngine::new(Box::new(PriceTimeStrategy));
    assert!(engine.start());
    loop {
        tokio::select! {
            command = receiver.recv() => {
                match command {
                    Some(command) => {
                        match command {
                            OrderCommand::Add { order, response } => {
                                let result = engine.process_order(order);

                                match result {
                                    Ok(result) => {
                                        let _ = response.send(
                                            OrderResponse::OrderProcessed(result)
                                        );
                                    }

                                    Err(error) => {
                                        println!("{}", error);

                                        let _ = response.send(
                                            OrderResponse::OrderError(error)
                                        );
                                    }
                                }
                            }
                        }
                    }

                    None => {
                        println!("All order senders dropped");
                        break;
                    }
                }
            }

           _ = &mut shutdown => {
            println!("Shutdown signal received");

            receiver.close();

            while let Some(command) = receiver.recv().await {
                match command {
                    OrderCommand::Add { order, response } => {
                        let result = engine.process_order(order);

                        match result {
                            Ok(result) => {
                                let _ = response.send(
                                    OrderResponse::OrderProcessed(result)
                                );
                            }

                            Err(error) => {
                                println!("{}", error);

                                let _ = response.send(
                                    OrderResponse::OrderError(error)
                                );
                            }
                        }
                    }
                }
            }

            break;
        }
        }
    }

    println!("OrderBook processor stopped");
}