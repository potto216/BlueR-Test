//! Client implementation.

use crate::rpc::{BlueRTest, BlueRTestClient};
use anyhow::{bail, Context, Result};
use bluer::AdapterEvent;
use bluer::{Uuid, UuidExt};
use clap::Parser;
use futures::{pin_mut, StreamExt};
use remoc::prelude::*;
use std::{collections::BTreeMap, time::Duration, vec::Vec};
use tokio::{net::TcpStream, time::sleep};
use tracing::{instrument};

#[derive(Parser, Debug)]
pub struct ClientOpts {
    /// Server hostname or IP address.
    #[clap(short, long, default_value = "localhost")]
    server: String,
    /// Test to perform.
    #[clap(subcommand)]
    test: Test,
}

#[derive(Parser, Debug)]
pub enum Test {
    /// Prints the server's Bluetooth address.
    ServerAddress,
    /// Performs the advertising test.
    AdvertisingServiceUUIDS128,
    AdvertisingServiceUUIDS16,
    AdvertisingServiceData,
    GattServer,
    KillServer,
}

#[instrument]
pub async fn run_client(verbose_mode: bool, port: u16, opts: ClientOpts) -> Result<()> {
    let socket = TcpStream::connect((opts.server.as_str(), port))
        .await
        .context("cannot connect to server")?;
    let (socket_rx, socket_tx) = socket.into_split();

    let client: BlueRTestClient = remoc::Connect::io(remoc::Cfg::default(), socket_rx, socket_tx)
        .consume()
        .await
        .context("cannot establish remoc connection")?;

    match opts.test {
        Test::ServerAddress => server_address(client, verbose_mode).await,
        Test::AdvertisingServiceUUIDS128 => advertising_test(client, verbose_mode, 128, 0).await,
        Test::AdvertisingServiceUUIDS16 => advertising_test(client, verbose_mode, 16, 0).await,
        Test::AdvertisingServiceData => advertising_test(client, verbose_mode, 16, 8).await,
        Test::GattServer => gatt_server_test(client, verbose_mode).await,
        Test::KillServer => request_kill_server(client, verbose_mode).await,
    }
}
#[instrument]
async fn server_address(client: BlueRTestClient, verbose_mode: bool) -> Result<()> {
    let server_addr = bluer::Address(client.get_server_address().await?);
    let client_addr = bluer::Address(client.get_client_address().await?);
    if verbose_mode {
    println!(
        "The server has Bluetooth address {server_addr}. The client address is {client_addr}",
        server_addr = server_addr,
        client_addr = client_addr
    );
}
    Ok(())
}

#[instrument]
async fn advertising_test(
    client: BlueRTestClient,
    verbose_mode: bool,
    uuid_length: u32,
    service_data_length: u32,
) -> Result<()> {
    let server_addr = bluer::Address(client.get_server_address().await?);

    let service_uuid = match uuid_length {
        128 => Uuid::new_v4(),
        16 => Uuid::from_u16(0x1800),
        invalid_size => panic!(
            "Invalid size of {invalid_size}",
            invalid_size = invalid_size
        ),
    };

    let name: u64 = rand::random();
    let name = format!("{name:016x}", name = name);

    if verbose_mode {
        println!("Server {server_addr} sending advertisement with name {name} and service uuid {service_uuid}",server_addr=server_addr, name=name,service_uuid=service_uuid);
    }

    /*
        let _stop_adv = client
        .advertise_service_uuids(Some(name.clone()), [service_uuid].into())
        .await
        .context("cannot send advertisement")?;
    */

    let service_data_random_bytes: Vec<u8> = (0..8).map(|_| rand::random::<u8>()).collect();
    let _stop_adv = if service_data_length == 0 {
        client
            .advertise_service_uuids(Some(name.clone()), [service_uuid].into())
            .await
            .context("cannot send advertisement")?
    } else {
        println!("{:?}", service_data_random_bytes);
        let mut service_data: BTreeMap<Uuid, Vec<u8>> = BTreeMap::new();
        service_data.insert(service_uuid, service_data_random_bytes.clone());

        client
            .advertise_service_data(Some(name.clone()), service_data)
            .await
            .context("cannot send advertisement")?
    };

    let session = bluer::Session::new().await?;
    //let adapter = session.default_adapter().await?;
    let adapter = session.adapter(&client.get_client_name().await?).unwrap();
    adapter.set_discoverable(true).await?;
    let mut disco = adapter.discover_devices_with_changes().await?;

    if verbose_mode {
        println!(
            "Client {client_addr} looking for  advertisement",
            client_addr = adapter.address().await.unwrap()
        );
    }

    let timeout = sleep(Duration::from_secs(20));
    pin_mut!(timeout);
    let mut received_service_data_valid = false;

    loop {
        let evt = tokio::select! {
            Some(evt) = disco.next() => evt,
            () = &mut timeout => bail!("timeout reached"),
        };

        match evt {
            AdapterEvent::DeviceAdded(addr) if addr == server_addr => {
                let device = adapter.device(addr)?;

                let dev_name = device.name().await?.unwrap();
                println!("name {} ", dev_name);

                let mut uuid_present = false;
                print!("**c1");

                if service_data_length > 0 {
                    match device.service_data().await? {
                        Some(service_data) => {
                            print!("**c2");
                            for (service_data_uuid, service_data_value) in service_data.iter() {
                                println!(
                                    "service uuid {} / data {:x?}",
                                    service_data_uuid, service_data_value
                                );
                                if received_service_data_valid == false {
                                    let matching = service_data_value
                                        .iter()
                                        .zip(&service_data_random_bytes)
                                        .filter(|&(a, b)| a == b)
                                        .count();
                                    if matching == service_data_random_bytes.len() {
                                        received_service_data_valid = true;
                                        if service_data_uuid == &service_uuid {
                                            uuid_present = true;
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            print!("No service data found.");
                        }
                    }
                } else {
                    if let Some(uuids) = device.uuids().await? {
                        if verbose_mode {
                            //let uuid_vec = uuids.into_iter().collect::<Vec<_>>();
                            let uuid_vec = uuids.iter().map(|n| n.to_string()).collect::<Vec<_>>();
                            println!("uuids {} for address {}", uuid_vec.join(","), addr);
                        }

                        if uuids.contains(&service_uuid) {
                            uuid_present = true;
                        }
                    }
                }
                print!("**c3");

                let name_match = dev_name == name.clone();

                //println!("uuids {} for address {}", service_data);

                if verbose_mode {
                    dbg!(uuid_present);
                    dbg!(name_match);
                    dbg!(received_service_data_valid);
                }

                if service_data_length > 0 {
                    if received_service_data_valid && name_match {
                        break;
                    }
                } else {
                    if uuid_present && name_match {
                        break;
                    }
                }
            }
            _ => (),
        }
    }

    Ok(())
}

#[instrument]
async fn gatt_server_test(client: BlueRTestClient, verbose_mode: bool) -> Result<()> {
    let server_addr = bluer::Address(client.get_server_address().await?);

    let service_uuid = Uuid::new_v4();

    let name: u64 = rand::random();
    let name = format!("{name:016x}", name = name);

    if verbose_mode {
        println!("Server {server_addr} sending advertisement with name {name} and service uuid {service_uuid}",server_addr=server_addr, name=name,service_uuid=service_uuid);
    }

    let _stop_adv = client
        .advertise_service_uuids(Some(name.clone()), [service_uuid].into())
        .await
        .context("cannot send advertisement")?;

    let session = bluer::Session::new().await?;

    let adapter = session.adapter(&client.get_client_name().await?).unwrap();
    let mut disco = adapter.discover_devices_with_changes().await?;

    if verbose_mode {
        println!(
            "Client {client_addr} looking for  advertisement",
            client_addr = adapter.address().await.unwrap()
        );
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

                let dev_name = match device.name().await {
                    Ok(name_option) => match name_option {
                        Some(name) => name,
                        None => String::from("<None>"),
                    },
                    Err(_) => String::from("<Error>"),
                };
                if verbose_mode {
                    println!("Found server {} advertising with name {} ", addr, dev_name);
                    println!("    Icon:               {:?}", device.icon().await?);
                    println!("    Class:              {:?}", device.class().await?);
                    println!(
                        "    UUIDs:              {:?}",
                        device.uuids().await?.unwrap_or_default()
                    );
                    println!("    Paried:             {:?}", device.is_paired().await?);
                    println!("    Connected:          {:?}", device.is_connected().await?);
                    println!("    Trusted:            {:?}", device.is_trusted().await?);
                    println!("    Modalias:           {:?}", device.modalias().await?);
                    println!("    RSSI:               {:?}", device.rssi().await?);
                    println!("    TX power:           {:?}", device.tx_power().await?);

                    println!("First connecting...");
                }
                let mut retries = 2;
                loop {
                    match device.connect().await {
                        Ok(()) => {
                            if verbose_mode {
                                println!("Connected");
                            }

                            break;
                        }
                        Err(err) if retries > 0 => {
                            if verbose_mode {
                                println!("Connect error: {}", &err);
                            }
                            retries -= 1;
                        }
                        _ => break,
                    }
                }

                loop {
                    match device.pair().await {
                        Ok(()) => {
                            if verbose_mode {
                                println!("Paired");
                            }
                            break;
                        }
                        Err(err) if retries > 0 => {
                            if verbose_mode {
                                println!("Pair error: {}", &err);
                            }
                            retries -= 1;
                        }
                        _ => break,
                    }
                }
                if verbose_mode {
                    println!("Found server {} advertising with name {} ", addr, dev_name);
                    println!("    Icon:               {:?}", device.icon().await?);
                    println!("    Class:              {:?}", device.class().await?);
                    println!(
                        "    UUIDs:              {:?}",
                        device.uuids().await?.unwrap_or_default()
                    );
                    println!("    Paried:             {:?}", device.is_paired().await?);
                    println!("    Connected:          {:?}", device.is_connected().await?);
                    println!("    Trusted:            {:?}", device.is_trusted().await?);
                    println!("    Modalias:           {:?}", device.modalias().await?);
                    println!("    RSSI:               {:?}", device.rssi().await?);
                    println!("    TX power:           {:?}", device.tx_power().await?);
                }

                if !device.is_connected().await? {
                    if verbose_mode {
                        println!("Connecting...");
                    }
                    let mut retries = 2;
                    loop {
                        match device.connect().await {
                            Ok(()) => break,
                            Err(err) if retries > 0 => {
                                if verbose_mode {
                                    println!("Connect error: {}", &err);
                                }
                                retries -= 1;
                            }
                            _ => break,
                        }
                    }

                    if !device.is_connected().await? {
                        if verbose_mode {
                            println!("{} Failed to connect.", dev_name);
                        }
                    } else {
                        if verbose_mode {
                            println!("{} Connected! Attempting to pair", dev_name);
                        }
                        loop {
                            match device.pair().await {
                                Ok(()) => break,
                                Err(err) => {
                                    if verbose_mode {
                                        println!("Connect error: {}", &err);
                                    }
                                }
                            }
                        }
                    }
                } else {
                    if verbose_mode {
                        println!("Device already connected.");
                    }
                }
            }
            _ => (),
        }
    }
}

#[instrument]
async fn request_kill_server(mut client: BlueRTestClient, verbose_mode: bool) -> Result<()> {
    let server_addr = bluer::Address(client.get_server_address().await?);

    if verbose_mode {
        println!("Server {server_addr} shutting down.");
    }

    let _stop_adv = client.kill_server().await.context("cannot kill server")?;

    Ok(())
}
