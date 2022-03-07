//! Testing server implementation.

use anyhow::{Context, Result};
use bluer::{adv::Advertisement, Uuid};
use remoc::{codec, prelude::*};
use std::{collections::BTreeSet, collections::BTreeMap, net::Ipv4Addr, vec::Vec};
use tokio::net::TcpListener;

use crate::rpc::{BlueRTest, BlueRTestServer, GenericRpcResult};

/// Runs the server.
pub async fn run_server(debug: bool, port: u16) -> Result<()> {

    let mut used_address_vec:Vec<String> = Vec::new();

    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port))
        .await
        .context("cannot listen")?;


    let session = bluer::Session::new()
    .await
    .context("cannot start BlueR session")?;

    
    let server_adapter_original = session.default_adapter().await.unwrap();
    used_address_vec.push(server_adapter_original.address().await.unwrap().to_string());
    if debug {
        println!("Adding server to used address list. The list is:");
        for used_address in &used_address_vec {
            println!("{used_address}",used_address=used_address);
        }
      }
   loop {
        let server_adapter = server_adapter_original.clone();
        
        println!("Waiting for connection on port {}", port);
        let (socket, addr) = listener.accept().await.context("cannot accept")?;
        let (socket_rx, socket_tx) = socket.into_split();

        println!("Accepted connection from {}", addr);

        let adapter_names = session.adapter_names().await.unwrap();
        let adapter_name = adapter_names.first().expect("No Bluetooth adapter present");
        let mut client_adapter = session.adapter(adapter_name).unwrap();
        let mut address_used: bool = false;
        for adapter_name in adapter_names {
            address_used = false;
            let adapter_tmp = session.adapter(&adapter_name).unwrap();
            println!("*1");
            let address = adapter_tmp.address().await.unwrap();
            if debug {
                println!("Checking Bluetooth adapter {} with address of {}:", &adapter_name, address);
                }    
            println!("*2");
            for used_name in &used_address_vec {
                println!("*3");
                if  &address.to_string() == used_name {
                    println!("Checking Bluetooth adapter {} with address of {}:", &adapter_name, address);
                    println!("*4");
                    address_used = true;
                    break;
                }                
            }
            println!("*5 address_used = {}", address_used);
            if  address_used == false {
                println!("*6");
                used_address_vec.push(address.to_string());
                client_adapter =  adapter_tmp;
                break;
            }
         }
    
         if  address_used == true {
            println!("Error, no free adapters.");
        }

        let session = bluer::Session::new()
            .await
            .context("cannot start BlueR session")?;
        let client_address = client_adapter.address().await.unwrap().to_string();
        let test_obj = BlueRTestObj { debug, session, server_adapter, client_adapter };

        let (server, client) = BlueRTestServer::<_, codec::Default>::new(test_obj, 1);
        if debug {
        println!("Calling remoc connect.");
        }
        remoc::Connect::io(remoc::Cfg::default(), socket_rx, socket_tx)
            .provide(client)
            .await
            .context("cannot establish remoc connection")?;
        if debug {
            println!("Calling server.serve().await;");
        }
        server.serve().await;
        if debug {
        println!("Finished calling server.serve().await;");
        }

        if debug {
            println!("Before removal the used address list is:");
            for used_address in &used_address_vec {
                println!("{used_address}",used_address=used_address);
            }
          }
        //Free the used address
        used_address_vec.retain(|x| *x != client_address);
        if debug {
            println!("Removed client address {client_address} from the used address list. Current used address list is:", client_address=client_address);
            for used_address in &used_address_vec {
                println!("{used_address}",used_address=used_address);
            }
        }
        
        
    }
}

/// Server object for the testing service.
/// each test will have one server and one client. Set this up at the time of creation
pub struct BlueRTestObj {
    debug: bool,
    session: bluer::Session,
    server_adapter:  bluer::Adapter,
    client_adapter:  bluer::Adapter,
}

/// Implementation of the remote testing service.
#[rtc::async_trait]
impl BlueRTest for BlueRTestObj {
    async fn get_server_address(&self) -> GenericRpcResult<[u8; 6]> {
        /*
        let adapter = self
            .session
            .default_adapter()
            .await
            .map_err(anyhow::Error::from)?; */
        let adapter = self.server_adapter.clone();
        let addr = adapter.address().await.map_err(anyhow::Error::from)?;
        Ok(addr.0)
    }

    async fn get_client_address(&self) -> GenericRpcResult<[u8; 6]> {
        /*
        let adapter = self
            .session
            .default_adapter()
            .await
            .map_err(anyhow::Error::from)?; */
        let adapter = self.client_adapter.clone();
        let addr = adapter.address().await.map_err(anyhow::Error::from)?;
        Ok(addr.0)
    }

    async fn get_client_name(&self) -> GenericRpcResult<String> {
        /*
        let adapter = self
            .session
            .default_adapter()
            .await
            .map_err(anyhow::Error::from)?; */
        let adapter = self.client_adapter.clone();
        let adapter_name = adapter.name().to_string();
        Ok(adapter_name)
    }

    async fn advertise_service_uuids(
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
            println!("Server {address:?} sending advertisement {adv:?}", adv=adv, address=adapter.address().await.unwrap());
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


    async fn advertise_service_data(
        &self,
        local_name: Option<String>,
        service_data: BTreeMap<Uuid, Vec<u8>>,
    ) -> GenericRpcResult<rch::oneshot::Sender<()>> {
        let adv = Advertisement {
            advertisement_type: bluer::adv::Type::Peripheral,
            service_data,
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
            println!("Server {address:?} sending advertisement {adv:?}", adv=adv, address=adapter.address().await.unwrap());
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


