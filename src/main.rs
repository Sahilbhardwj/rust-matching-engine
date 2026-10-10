
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinSet;
use tokio_tungstenite::{accept_async, tungstenite::Message};

use orderbook_modules::{
    command::OrderCommand,
    order::{Order, Side},
    processor::run_orderbook,
};

#[derive(Debug, Deserialize)]
struct AddOrderRequest {
    id: u64,
    price: f64,
    quantity: f64,
    side: Side,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bind first so we don't start the processor if the
    // server cannot listen on the requested address.
    let listener = TcpListener::bind("127.0.0.1:9001").await?;

    println!("Server listening on ws://127.0.0.1:9001");

    // WebSocket handlers -> MPSC -> OrderBook processor
    let (sender, receiver) = mpsc::channel(100);

    // OrderBook processor shutdown signal.
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    // Broadcast shutdown notification to WebSocket handlers.
    let (connection_shutdown_tx, connection_shutdown_rx) =
        watch::channel(false);

    // Start the single-owner OrderBook processor.
    let orderbook_task = tokio::spawn(async move {
        run_orderbook(receiver, shutdown_rx).await;
    });

    // Track every WebSocket connection task.
    let mut connection_tasks = JoinSet::new();

    // Create the Ctrl+C future once.
    let shutdown_signal = tokio::signal::ctrl_c();
    tokio::pin!(shutdown_signal);

    // Main server loop.
    loop {
        tokio::select! {
            // Accept new TCP connections.
            result = listener.accept() => {
                let (stream, address) = match result {
                    Ok(connection) => connection,
                    Err(error) => {
                        eprintln!("Failed to accept connection: {error}");
                        continue;
                    }
                };

                println!("Client connected: {address}");

                let sender = sender.clone();
                let mut connection_shutdown =
                    connection_shutdown_rx.clone();

                // Track the connection task in JoinSet.
                connection_tasks.spawn(async move {
                    // Upgrade TCP to WebSocket.
                    let websocket = match accept_async(stream).await {
                        Ok(websocket) => websocket,
                        Err(error) => {
                            eprintln!(
                                "WebSocket handshake failed: {error}"
                            );
                            return;
                        }
                    };

                    println!("WebSocket established: {address}");

                    let (mut write, mut read) = websocket.split();

                    // Handle messages until disconnect or shutdown.
                    loop {
                        tokio::select! {
                            // Receive the next WebSocket message.
                            message = read.next() => {
                                let Some(message) = message else {
                                    break;
                                };

                                let message = match message {
                                    Ok(message) => message,
                                    Err(error) => {
                                        eprintln!(
                                            "WebSocket error: {error}"
                                        );
                                        break;
                                    }
                                };

                                // This server accepts text JSON messages.
                                if !message.is_text() {
                                    continue;
                                }

                                let text = match message.to_text() {
                                    Ok(text) => text,
                                    Err(error) => {
                                        eprintln!(
                                            "Could not read message: {error}"
                                        );
                                        continue;
                                    }
                                };

                                println!("Received: {text}");

                                // JSON -> request.
                                let request: AddOrderRequest =
                                    match serde_json::from_str(text) {
                                        Ok(request) => request,
                                        Err(error) => {
                                            eprintln!(
                                                "Invalid JSON: {error}"
                                            );
                                            continue;
                                        }
                                    };

                                // Request -> order.
                                let order = Order {
                                    id: request.id,
                                    price: request.price,
                                    quantity: request.quantity,
                                    side: request.side,
                                };

                                // Create a response channel.
                                let (response_tx, response_rx) =
                                    oneshot::channel();

                                // Order -> command.
                                let command = OrderCommand::Add {
                                    order,
                                    response: response_tx,
                                };

                                // Submit the order to the processor.
                                // Once submitted, finish waiting for its
                                // response even if shutdown is requested.
                                if sender.send(command).await.is_err() {
                                    eprintln!(
                                        "OrderBook processor stopped"
                                    );
                                    break;
                                }

                                let response = match response_rx.await {
                                    Ok(response) => response,
                                    Err(_) => {
                                        eprintln!(
                                            "Processor dropped response channel"
                                        );
                                        break;
                                    }
                                };

                                println!(
                                    "Processor response: {response:?}"
                                );

                                // Response -> JSON.
                                let json =
                                    match serde_json::to_string(&response) {
                                        Ok(json) => json,
                                        Err(error) => {
                                            eprintln!(
                                                "Serialization failed: {error}"
                                            );
                                            continue;
                                        }
                                    };

                                // JSON -> WebSocket client.
                                if let Err(error) =
                                    write.send(Message::Text(json.into())).await
                                {
                                    eprintln!(
                                        "Failed to send response: {error}"
                                    );
                                    break;
                                }
                            }

                            // Observe the broadcast shutdown signal.
                            result = connection_shutdown.changed() => {
                                if result.is_err()
                                    || *connection_shutdown.borrow_and_update()
                                {
                                    println!(
                                        "Shutdown requested for {address}"
                                    );
                                    break;
                                }
                            }
                        }
                    }

                    println!("Client disconnected: {address}");
                });
            }

            // Collect completed connection tasks while running.
            Some(result) = connection_tasks.join_next(),
                if !connection_tasks.is_empty() =>
            {
                match result {
                    Ok(()) => {
                        println!("WebSocket task completed");
                    }
                    Err(error) => {
                        eprintln!("WebSocket task failed: {error}");
                    }
                }
            }

            // Ctrl+C initiates coordinated shutdown.
            signal_result = &mut shutdown_signal => {
                if let Err(error) = signal_result {
                    eprintln!("Failed to listen for Ctrl+C: {error}");
                }

                println!("Shutdown signal received");

                // Tell all active WebSocket handlers to stop
                // accepting new messages.
                let _ = connection_shutdown_tx.send(true);

                // Stop accepting new TCP connections.
                break;
            }
        }
    }

    // Wait for all active WebSocket handlers to finish.
    // In-flight orders can still receive processor responses.
    while let Some(result) = connection_tasks.join_next().await {
        match result {
            Ok(()) => println!("WebSocket task completed"),
            Err(error) => {
                eprintln!("WebSocket task failed: {error}");
            }
        }
    }

    // All tracked connection handlers have exited.
    // Now tell the processor to drain its accepted commands.
    let _ = shutdown_tx.send(());

    // Wait for the OrderBook processor to finish.
    match orderbook_task.await {
        Ok(()) => {
            println!("OrderBook worker exited normally");
        }
        Err(error) if error.is_panic() => {
            eprintln!("OrderBook worker panicked: {error}");
        }
        Err(error) if error.is_cancelled() => {
            eprintln!("OrderBook worker was cancelled: {error}");
        }
        Err(error) => {
            eprintln!("OrderBook worker failed: {error}");
        }
    }

    println!("Server shut down");

    Ok(())
}
