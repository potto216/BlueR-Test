use std::net::Ipv4Addr;
use tokio::net::{TcpStream};
use remoc::prelude::*;


#[tokio::main]
async fn main() {
    // For demonstration we run both client and server in
    // the same process. In real life connect_client() and
    // connect_server() would run on different machines.
    connect_client().await;
    println!("The client ran!");
}

// This would be run on the client.
// It establishes a Remoc connection over TCP to the server.
async fn connect_client() {
    // Wait for server to be ready.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Establish TCP connection.
    let socket =
        TcpStream::connect((Ipv4Addr::LOCALHOST, 9870)).await.unwrap();
    let (socket_rx, socket_tx) = socket.into_split();

    // Establish Remoc connection over TCP.
    // The connection is always bidirectional, but we can just drop
    // the unneeded receiver.
    let (conn, tx, _rx): (_, _, rch::base::Receiver<()>) =
        remoc::Connect::io(remoc::Cfg::default(), socket_rx, socket_tx)
        .await.unwrap();
    tokio::spawn(conn);

    // Run client.
    client(tx).await;
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

// This would be run on the client.
// It sends a count request to the server and receives each number
// as it is counted over a newly established MPSC channel.
async fn client(mut tx: rch::base::Sender<CountReq>) {
    // By sending seq_tx over an existing remote channel, a new remote
    // channel is automatically created and connected to the server.
    // This all happens inside the existing TCP connection.
    let (seq_tx, mut seq_rx) = rch::mpsc::channel(1);
    tx.send(CountReq { up_to: 4, seq_tx }).await.unwrap();

    // Receive counted numbers over new channel.
    assert_eq!(seq_rx.recv().await.unwrap(), Some(0));
    assert_eq!(seq_rx.recv().await.unwrap(), Some(1));
    assert_eq!(seq_rx.recv().await.unwrap(), Some(2));
    assert_eq!(seq_rx.recv().await.unwrap(), Some(3));
    assert_eq!(seq_rx.recv().await.unwrap(), None);
}

