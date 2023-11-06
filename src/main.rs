

use futures_util::stream::FuturesUnordered;
use futures_util::{SinkExt, StreamExt};

use std::ops::ControlFlow;
use std::time::Instant;
 
// we will use tungstenite for websocket client impl (same library as what axum is using)
use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::{frame::coding::CloseCode, CloseFrame, Message},
};
 
const N_CLIENTS: usize = 1; //set to desired number
const SERVER: &str = "ws://127.0.0.1:3000/ws/2";
 
#[tokio::main]
async fn main() {
    let start_time = Instant::now();
    //spawn several clients that will concurrently talk to the server
    let mut clients = (0..N_CLIENTS)
        .map(|cli| tokio::spawn(spawn_client(cli)))
        .collect::<FuturesUnordered<_>>();
 
    //wait for all our clients to exit
    while let Some(_) = clients.next().await {}
 
    let end_time = Instant::now();
    //total time should be the same no matter how many clients we spawn
    /* println!(
        "Total time taken {:#?} with {N_CLIENTS} concurrent clients, should be about 6.45 seconds.",
        end_time - start_time
    ); */
}
 
//creates a client. quietly exits on failure.
async fn spawn_client(who: usize) {
    let ws_stream = match connect_async(SERVER).await {
        Ok((stream, response)) => {
            println!("Handshake for client {who} has been completed");
            // This will be the HTTP response, same as with server this is the last moment we
            // can still access HTTP stuff.
            //println!("Server response was {response:?}");
            stream
        }
        Err(e) => {
            println!("WebSocket handshake for client {who} failed with {e}!");
            return;
        }
    };
 
    let (mut sender, mut receiver) = ws_stream.split();
 
    //spawn an async sender to push some more messages into the server
    let send_task = tokio::spawn(async move {
        loop {
            let mut input = String::new();
            println!("Enter your message or type 'exit' to quit:");
            std::io::stdin().read_line(&mut input).unwrap();
            let trimmed = input.trim();
            if trimmed.eq_ignore_ascii_case("exit") {
                break;
            }
            sender.send(Message::Text(trimmed.to_string())).await.unwrap();
        }
    });
 
    //receiver just prints whatever it gets
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            // print message and break if instructed to do so
            if process_message(msg, who).is_break() {
                break;
            }
        }
    });
 
    
}
 
/// Function to handle messages we get
fn process_message(msg: Message, who: usize) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            println!(">>> {who} got str: {t:?}");
        }
        Message::Binary(d) => {
            println!(">>> {} got {} bytes: {:?}", who, d.len(), d);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> {} got close with code {} and reason `{}`",
                    who, cf.code, cf.reason
                );
            } else {
                println!(">>> {who} somehow got close message without CloseFrame");
            }
            return ControlFlow::Break(());
        }
        Message::Pong(v) => {
            println!(">>> {who} got pong with {v:?}");
        }
        Message::Ping(v) => {
            println!(">>> {who} got ping with {v:?}");
        }
        Message::Frame(_) => {
            unreachable!("This is never supposed to happen")
        }
    }
    ControlFlow::Continue(())
}