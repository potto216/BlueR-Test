//! Client implementation.

use anyhow::{bail, Context, Result};
use bluer::{AdapterEvent, Uuid};
use clap::Parser;
use futures::{pin_mut, StreamExt};
use remoc::prelude::*;
use std::time::Duration;
use tokio::{net::TcpStream, time::sleep};

use crate::rpc::{BlueRTest, BlueRTestClient};

#[derive(Parser)]
pub struct ClientOpts {
    /// Server hostname or IP address.
    #[clap(short, long, default_value = "localhost")]
    server: String,
    /// Test to perform.
    #[clap(subcommand)]
    test: Test,
}

#[derive(Parser)]
pub enum Test {
    /// Prints the server's Bluetooth address.
    ServerAddress,
    /// Performs the advertising test.
    Advertising,
}

pub async fn run_client(debug: bool, port: u16, opts: ClientOpts) -> Result<()> {
    let socket = TcpStream::connect((opts.server.as_str(), port))
        .await
        .context("cannot connect to server")?;
    let (socket_rx, socket_tx) = socket.into_split();

    let client: BlueRTestClient = remoc::Connect::io(remoc::Cfg::default(), socket_rx, socket_tx)
        .consume()
        .await
        .context("cannot establish remoc connection")?;

    match opts.test {
        Test::ServerAddress => server_address(client, debug).await,
        Test::Advertising => advertising_test(client, debug).await,
    }
}

async fn server_address(client: BlueRTestClient, _debug: bool) -> Result<()> {
    let addr = bluer::Address(client.get_server_address().await?);
    println!("The server has Bluetooth address {addr}",addr=addr);
    Ok(())
}

async fn advertising_test(client: BlueRTestClient, debug: bool) -> Result<()> {
    let server_addr = bluer::Address(client.get_server_address().await?);

    let service_uuid = Uuid::new_v4();
    let name: u64 = rand::random();
    let name = format!("{name:016x}",name=name);

    if debug {
        println!("Server {server_addr} sending advertisement with name {name} and service uuid {service_uuid}",server_addr=server_addr, name=name,service_uuid=service_uuid);
    }
    let _stop_adv = client
        .advertise(Some(name.clone()), [service_uuid].into())
        .await
        .context("cannot send advertisement")?;

    
    let session = bluer::Session::new().await?;
    //let adapter = session.default_adapter().await?;
    let adapter = session.adapter(&client.get_client_name().await?).unwrap();   
    let mut disco = adapter.discover_devices_with_changes().await?;

    if debug {
        println!("Client {client_addr} looking for  advertisement",client_addr=adapter.address().await.unwrap());
    }


    let timeout = sleep(Duration::from_secs(20));
    pin_mut!(timeout);

    loop {
        let evt = tokio::select! {
            Some(evt) = disco.next() => evt,
            () = &mut timeout => bail!("timeout reached"),
        };

        match evt {
            AdapterEvent::DeviceAdded(addr) if addr == server_addr => {
                if debug {
                    println!("Server device found");
                }

                let device = adapter.device(addr)?;

                let mut uuid_present = false;
                if let Some(uuids) = device.uuids().await? {
                    if uuids.contains(&service_uuid) {
                        uuid_present = true;
                    }
                }

                let dev_name = device.name().await?;
                let name_match = dev_name == Some(name.clone());

                if debug {
                    dbg!(uuid_present);
                    dbg!(name_match);
                }

                if uuid_present && name_match {
                    break;
                }
            }
            _ => (),
        }
    }

    Ok(())
}
