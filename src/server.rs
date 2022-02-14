//! Testing server implementation.

use anyhow::{Context, Result};
use bluer::{adv::Advertisement, Uuid};
use remoc::{codec, prelude::*};
use std::{collections::BTreeSet, net::Ipv4Addr};
use tokio::net::TcpListener;

use crate::rpc::{BlueRTest, BlueRTestServer, GenericRpcResult};

/// Runs the server.
pub async fn run_server(debug: bool, port: u16) -> Result<()> {
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port))
        .await
        .context("cannot listen")?;

    loop {
        println!("Waiting for connection on port {}", port);
        let (socket, addr) = listener.accept().await.context("cannot accept")?;
        let (socket_rx, socket_tx) = socket.into_split();

        println!("Accepted connection from {}", addr);

        let session = bluer::Session::new()
            .await
            .context("cannot start BlueR session")?;
        let test_obj = BlueRTestObj { debug, session };

        let (server, client) = BlueRTestServer::<_, codec::Default>::new(test_obj, 1);
        remoc::Connect::io(remoc::Cfg::default(), socket_rx, socket_tx)
            .provide(client)
            .await
            .context("cannot establish remoc connection")?;
        server.serve().await;
    }
}

/// Server object for the testing service.
pub struct BlueRTestObj {
    debug: bool,
    session: bluer::Session,
}

/// Implementation of the remote testing service.
#[rtc::async_trait]
impl BlueRTest for BlueRTestObj {
    async fn get_address(&self) -> GenericRpcResult<[u8; 6]> {
        let adapter = self
            .session
            .default_adapter()
            .await
            .map_err(anyhow::Error::from)?;
        let addr = adapter.address().await.map_err(anyhow::Error::from)?;
        Ok(addr.0)
    }

    async fn advertise(
        &self,
        local_name: Option<String>,
        service_uuids: BTreeSet<Uuid>,
    ) -> GenericRpcResult<rch::oneshot::Sender<()>> {
        let adv = Advertisement {
            advertisement_type: bluer::adv::Type::Peripheral,
            service_uuids,
            discoverable: Some(true),
            local_name,
            ..Default::default()
        };

        let adapter = self
            .session
            .default_adapter()
            .await
            .map_err(anyhow::Error::from)?;
        adapter
            .set_powered(true)
            .await
            .map_err(anyhow::Error::from)?;

        if self.debug {
            println!("Sending advertisement {adv:?}");
        }
        let hndl = adapter.advertise(adv).await.map_err(anyhow::Error::from)?;

        let (stop_tx, stop_rx) = rch::oneshot::channel();
        let debug = self.debug;
        tokio::spawn(async move {
            let _ = stop_rx.await;

            if debug {
                println!("Stop sending advertisement");
            }
            drop(hndl);
        });

        Ok(stop_tx)
    }
}
