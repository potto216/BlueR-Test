use std::net::Ipv4Addr;
use tokio::net::{TcpListener};
use remoc::prelude::*;

#[tokio::main]
async fn main() {
    // For demonstration we run both client and server in
    // the same process. In real life connect_client() and
    // connect_server() would run on different machines.
     connect_server().await;
     println!("The server ran!");
}



// This would be run on the server.
// It accepts a Remoc connection over TCP from the client.
async fn connect_server() {
    // Listen for incoming TCP connection.
    let listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 9870)).await.unwrap();
    let (socket, _) = listener.accept().await.unwrap();
    let (socket_rx, socket_tx) = socket.into_split();

    // Establish Remoc connection over TCP.
    // The connection is always bidirectional, but we can just drop
    // the unneeded sender.
    let (conn, _tx, rx): (_, rch::base::Sender<()>, _) =
        remoc::Connect::io(remoc::Cfg::default(), socket_rx, socket_tx)
        .await.unwrap();
    tokio::spawn(conn);

    // Run server.
    server(rx).await;
}

// User-defined data structures needs to implement Serialize
// and Deserialize.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CountReq {
    up_to: u32,
    // Most Remoc types like channels can be included in serializable
    // data structures for transmission to remote endpoints.
    seq_tx: rch::mpsc::Sender<u32>,
}


// This would be run on the server.
// It receives a count request from the client and sends each number
// as it is counted over the MPSC channel sender provided by the client.
async fn server(mut rx: rch::base::Receiver<CountReq>) {
    // Receive count request and channel sender to use for counting.
    while let Some(CountReq {up_to, seq_tx}) = rx.recv().await.unwrap()
    {
        for i in 0..up_to {
            // Send each counted number over provided channel.
            seq_tx.send(i).await.unwrap();
        }
    }
}