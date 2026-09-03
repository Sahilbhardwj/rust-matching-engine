use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;

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
async fn main()-> Result<(), Box<dyn std::error::Error>>{
    // ------------------------------------
    // MAIN MPSC CHANNEL
    //
    // WebSocket tasks
    //       ↓
    //      MPSC
    //       ↓
    // OrderBook processor
    // ------------------------------------

    let (sender, receiver) = mpsc::channel(100);

    // Start the single OrderBook processor.
    tokio::spawn(async move {
        run_orderbook(receiver).await;
    });

    // ------------------------------------
    // TCP LISTENER
    // ------------------------------------

    let listener = TcpListener::bind("127.0.0.1:9001")
        .await?;

    println!("Server listening on ws://127.0.0.1:9001");

    loop {
        let (stream, address) = listener
            .accept()
            .await
            .unwrap();

        println!("Client connected: {}", address);

        // Every WebSocket task gets its own clone
        // of the MPSC sender.
        let sender = sender.clone();

        tokio::spawn(async move {
            // ------------------------------------
            // TCP → WEBSOCKET
            // ------------------------------------

            let websocket = match accept_async(stream).await {
                Ok(websocket) => websocket,

                Err(error) => {
                    println!(
                        "WebSocket handshake failed: {}",
                        error
                    );
                    return;
                }
            };

            println!(
                "WebSocket established: {}",
                address
            );

            let (mut write, mut read) = websocket.split();

            // ------------------------------------
            // READ MESSAGES FROM THIS CLIENT
            // ------------------------------------

            while let Some(message) = read.next().await {
                let message = match message {
                    Ok(message) => message,

                    Err(error) => {
                        println!(
                            "WebSocket error: {}",
                            error
                        );
                        break;
                    }
                };

                // We currently only handle text messages.
                if !message.is_text() {
                    continue;
                }

                let text = match message.to_text() {
                    Ok(text) => text,

                    Err(error) => {
                        println!(
                            "Could not read message: {}",
                            error
                        );
                        continue;
                    }
                };

                println!("Received: {}", text);

                // ------------------------------------
                // JSON → RUST REQUEST
                // ------------------------------------

                let request: AddOrderRequest =
                    match serde_json::from_str(text) {
                        Ok(request) => request,

                        Err(error) => {
                            println!(
                                "Invalid JSON: {}",
                                error
                            );
                            continue;
                        }
                    };

                // ------------------------------------
                // REQUEST → ORDER
                // ------------------------------------

                let order = Order {
                    id: request.id,
                    price: request.price,
                    quantity: request.quantity,
                    side: request.side,
                };

                // ------------------------------------
                // CREATE ONESHOT RESPONSE CHANNEL
                // ------------------------------------

                let (response_tx, response_rx) =
                    tokio::sync::oneshot::channel();

                // ------------------------------------
                // ORDER → ORDER COMMAND
                // ------------------------------------

                let command = OrderCommand::Add {
                    order,
                    response: response_tx,
                };

                // ------------------------------------
                // SEND COMMAND THROUGH MPSC
                // ------------------------------------

                if sender.send(command).await.is_err() {
                    println!("OrderBook processor stopped");
                    break;
                }

                // ------------------------------------
                // WAIT FOR PROCESSOR
                // ------------------------------------

                let response = match response_rx.await {
                    Ok(response) => response,

                    Err(_) => {
                        println!(
                            "Processor dropped response channel"
                        );
                        break;
                    }
                };

                println!(
                    "Processor response: {:?}",
                    response
                );

                // ------------------------------------
                // RESPONSE → JSON
                // ------------------------------------

                let json =
                    match serde_json::to_string(&response) {
                        Ok(json) => json,

                        Err(error) => {
                            println!(
                                "Failed to serialize response: {}",
                                error
                            );
                            continue;
                        }
                    };

                // ------------------------------------
                // JSON → WEBSOCKET CLIENT
                // ------------------------------------

                if let Err(error) =
                    write
                        .send(
                            tokio_tungstenite::tungstenite::Message::Text(
                                json.into(),
                            )
                        )
                        .await
                {
                    println!(
                        "Failed to send response: {}",
                        error
                    );
                    break;
                }
            }

            println!(
                "Client disconnected: {}",
                address
            );
        });
       
    }
  
}