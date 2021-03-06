//! Testing server implementation.

use anyhow::{Context, Result};
use bluer::{adv::Advertisement, Uuid, Adapter, Session};
use remoc::{codec, prelude::*};
use std::{collections::BTreeSet, collections::BTreeMap, net::Ipv4Addr, vec::Vec};
use tokio::net::TcpListener;
use tracing::{instrument};

use crate::rpc::{BlueRTest, BlueRTestServer, GenericRpcResult};


struct AdapterPool {
    used_addresses:Vec<String>
}
// impl of Val
impl AdapterPool {
    //This creates an empty database 
    fn new() -> AdapterPool {
        AdapterPool { used_addresses: Vec::<String>::new() }
    }

    //This adds a server to the database. It will allow multiple servers and will not rmeove one if it already exists. It will choose the first free adapter
    async fn free_adapter(&mut self, adapter:Adapter)  -> Result<bool> 
    { 
        let verbose_mode=true; 

        let address = adapter.address().await.unwrap().to_string();

        if verbose_mode {
            println!("Before removal the used address list is:");
            for used_address in &self.used_addresses {
                println!("{used_address}",used_address=used_address);
            }
          }

        //Free the used address
        self.used_addresses.retain(|x| *x != address);

        if verbose_mode {
            println!("Removed address {address} from the used address list. Current used address list is:", address=address);
            for used_address in &self.used_addresses {
                println!("{used_address}",used_address=used_address);
            }
        }

        Ok(true)
    }


    async fn get_adapter(&mut self, session:&Session, verbose_mode:bool ) -> Result<Adapter,String> 
    {
            let mut address_used: bool = false;

        let adapter_names = session.adapter_names().await.unwrap();
        for adapter_name in adapter_names {

            address_used = false;
            let adapter_try = session.adapter(&adapter_name).unwrap();
            let address = adapter_try.address().await.unwrap();
            if verbose_mode {
                println!("Checking Bluetooth adapter {} with address of {}:", &adapter_name, address);
                }    
            
            for used_address in &self.used_addresses {
            
                if  &address.to_string() == used_address {
                    if verbose_mode {
                    println!("Checking Bluetooth adapter {} with address of {}:", &adapter_name, address);                    
                    }
                    address_used = true;
                    break;
                }                
            }
            if  address_used == false {
                self.used_addresses.push(address.to_string());
                return Ok(adapter_try.clone());
            }
         }
    
         if  address_used == true {
            if verbose_mode {
            println!("Error, no free adapters.");
            }
            Err("No free adapters found.".to_string())
        }
        else{
            Err("Unknown error.".to_string())
        }

    }

}


/// Runs the server.
#[instrument]
pub async fn run_server(verbose_mode: bool, port: u16) -> Result<()> {

 //   let mut used_address_vec:Vec<String> = Vec::new();

    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port))
        .await
        .context("cannot listen")?;


    let session = bluer::Session::new()
    .await
    .context("cannot start BlueR session")?;

    let mut adapter_pool = AdapterPool::new();
    
    let server_adapter = adapter_pool.get_adapter(&session,verbose_mode).await.unwrap();
    
   loop {
        
        println!("Waiting for connection on port {}", port);
        let (socket, addr) = listener.accept().await.context("cannot accept")?;
        let (socket_rx, socket_tx) = socket.into_split();

        println!("Accepted connection from {}", addr);


        let client_adapter= adapter_pool.get_adapter(&session,verbose_mode).await.unwrap();

        let test_obj = BlueRTestObj { verbose_mode, kill_server_status: false, server_adapter: server_adapter.clone(), client_adapter: client_adapter.clone() };

        let (server, client) = BlueRTestServer::<_, codec::Default>::new(test_obj, 1);
        if verbose_mode {
        println!("Calling remoc connect.");
        }
        let res_back=remoc::Connect::io(remoc::Cfg::default(), socket_rx, socket_tx)
            .provide(client)
            .await
            .context("cannot establish remoc connection")?;

        if verbose_mode {
            println!("Result back {:?}",res_back);
            println!("Calling server.serve().await;");
        }
        let serve_result=server.serve().await.context("Serve failed")?;

        if verbose_mode {
        println!("Finished calling server.serve().await;");
        println!("Kill server status is {:?}", serve_result.get_kill_server_status().await.unwrap());
        }
   
        adapter_pool.free_adapter(client_adapter).await.unwrap();
        if serve_result.get_kill_server_status().await.unwrap()==true
        {
            break
        }
            
    }
    Ok(())
}

/// Server object for the testing service.
/// each test will have one server and one client. Set this up at the time of creation
pub struct BlueRTestObj {
    verbose_mode: bool,
    kill_server_status: bool,
    //session: bluer::Session,
    server_adapter:  bluer::Adapter,
    client_adapter:  bluer::Adapter,
}

/// Implementation of the remote testing service.
#[rtc::async_trait]
impl BlueRTest for BlueRTestObj {
    async fn get_server_address(&self) -> GenericRpcResult<[u8; 6]> {
       
        let adapter = self.server_adapter.clone();
        let addr = adapter.address().await.map_err(anyhow::Error::from)?;
        Ok(addr.0)
    }

    async fn get_client_address(&self) -> GenericRpcResult<[u8; 6]> {
        let adapter = self.client_adapter.clone();
        let addr = adapter.address().await.map_err(anyhow::Error::from)?;
        Ok(addr.0)
    }

    async fn get_client_name(&self) -> GenericRpcResult<String> {
        let adapter = self.client_adapter.clone();
        let adapter_name = adapter.name().to_string();
        Ok(adapter_name)
    }


    async fn run_gatt_server(& self) -> GenericRpcResult<bool> {        
        Ok(true)
    }


    async fn get_kill_server_status(& self) -> GenericRpcResult<bool> {        
        Ok(self.kill_server_status)
    }

    async fn kill_server(&mut self) -> GenericRpcResult<bool> {
        self.kill_server_status=true;
        Ok(self.kill_server_status)
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

        let adapter = self.server_adapter.clone();
            
        adapter
            .set_powered(true)
            .await
            .map_err(anyhow::Error::from)?;

        adapter.set_discoverable(true).await.unwrap();
        if self.verbose_mode {
            println!("Server {address:?} sending advertisement {adv:?}", adv=adv, address=adapter.address().await.unwrap());
        }
        let hndl = adapter.advertise(adv).await.map_err(anyhow::Error::from)?;

        let (stop_tx, stop_rx) = rch::oneshot::channel();
        let verbose_mode = self.verbose_mode;
        tokio::spawn(async move {
            let _ = stop_rx.await;

            if verbose_mode {
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

        let adapter = self.server_adapter.clone();
        adapter
            .set_powered(true)
            .await
            .map_err(anyhow::Error::from)?;

        adapter.set_discoverable(true).await.unwrap();
        if self.verbose_mode {
            println!("Server {address:?} sending advertisement {adv:?}", adv=adv, address=adapter.address().await.unwrap());
        }
        let hndl = adapter.advertise(adv).await.map_err(anyhow::Error::from)?;

        let (stop_tx, stop_rx) = rch::oneshot::channel();
        let verbose_mode = self.verbose_mode;
        tokio::spawn(async move {
            let _ = stop_rx.await;

            if verbose_mode {
                println!("Stop sending advertisement");
            }
            drop(hndl);
        });

        Ok(stop_tx)
    }    
}


