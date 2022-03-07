//! Client implementation.

use anyhow::{bail, Context, Result};
use bluer::{AdapterEvent, Uuid, UuidExt};
use clap::Parser;
use futures::{pin_mut, StreamExt};
use remoc::prelude::*;
use std::{time::Duration, collections::BTreeMap, vec::Vec};
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
    AdvertisingServiceUUIDS128,
    AdvertisingServiceUUIDS16,
    AdvertisingServiceData,

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
        Test::AdvertisingServiceUUIDS128 => advertising_test(client, debug, 128, 0).await,
        Test::AdvertisingServiceUUIDS16 => advertising_test(client, debug, 16, 0).await,
        Test::AdvertisingServiceData => advertising_test(client, debug, 16, 8).await,
    }
}

async fn server_address(client: BlueRTestClient, _debug: bool) -> Result<()> {
    let server_addr = bluer::Address(client.get_server_address().await?);
    let client_addr = bluer::Address(client.get_client_address().await?);
    println!("The server has Bluetooth address {server_addr}. The client address is {client_addr}",server_addr=server_addr, client_addr=client_addr);
    Ok(())
}

async fn advertising_test(client: BlueRTestClient, debug: bool, uuid_length: u32, service_data_length: u32) -> Result<()> {
    let server_addr = bluer::Address(client.get_server_address().await?);

    let service_uuid =match uuid_length {
        128 => Uuid::new_v4(),
        16 => Uuid::from_u16(0x1800),
        invalid_size  => panic!("Invalid size of {invalid_size}", invalid_size=invalid_size),
    };

    let name: u64 = rand::random();
    let name = format!("{name:016x}",name=name);

    if debug {
        println!("Server {server_addr} sending advertisement with name {name} and service uuid {service_uuid}",server_addr=server_addr, name=name,service_uuid=service_uuid);
    }


    /*
    let _stop_adv = client
    .advertise_service_uuids(Some(name.clone()), [service_uuid].into())
    .await
    .context("cannot send advertisement")?;
*/
    
    

    let _stop_adv =  if service_data_length == 0 {
        client
        .advertise_service_uuids(Some(name.clone()), [service_uuid].into())
        .await
        .context("cannot send advertisement")?
    } else
    {
        let random_bytes: Vec<u8> = (0..8).map(|_| { rand::random::<u8>() }).collect();
        println!("{:?}", random_bytes);
        let mut service_data: BTreeMap<Uuid, Vec<u8>> = BTreeMap::new();
        service_data.insert(service_uuid,  random_bytes);
    
        client
        .advertise_service_data(Some(name.clone()), service_data)
        .await
        .context("cannot send advertisement")?
    };
 
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
                let device = adapter.device(addr)?;


                let mut uuid_present = false;
                if let Some(uuids) = device.uuids().await? {

                    if debug {
                        //let uuid_vec = uuids.into_iter().collect::<Vec<_>>();
                        let uuid_vec = uuids.iter().map(|n| n.to_string()).collect::<Vec<_>>();
                        println!("uuids {} for address {}", uuid_vec.join(","), addr);
                    }

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
