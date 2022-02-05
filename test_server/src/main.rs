//! This crate implements the server of the remote counting service.
use bluer::adv::Advertisement;
use remoc::{codec, prelude::*};
use std::{net::Ipv4Addr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::RwLock, time::sleep};

use counter::{Counter, CounterServerSharedMut, TestCommand, TestCommandServerSharedMut, IncreaseError, TCP_PORT};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "test_server", about = "A server that clients can make requests to perform multiple Bluetooth tests")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short, long, help="Show additional information for troubleshooting such as details about the adapters")]
    debug: bool,
 
    // short and long flags (-a, --advertiser) will be deduced from the field's name     
    #[structopt(short, long, required=true, help="The Bluetooth controller address to use in the form XX:XX:XX:XX:XX:XX  ex: 5C:F3:70:A1:71:0F")]
    bluetooth_address: String,
}


/// Server object for the counting service, keeping the state.
#[derive(Default)]
pub struct CounterObj {
    /// The current value.
    value: u32,
    /// The subscribed watchers.
    watchers: Vec<rch::watch::Sender<u32>>,
}

/// Implementation of remote counting service.
#[rtc::async_trait]
impl Counter for CounterObj {
    async fn value(&self) -> Result<u32, rtc::CallError> {
        Ok(self.value)
    }

    async fn watch(&mut self) -> Result<rch::watch::Receiver<u32>, rtc::CallError> {
        // Create watch channel.
        let (tx, rx) = rch::watch::channel(self.value);
        // Keep the sender half in the watchers vector.
        self.watchers.push(tx);
        // And return the receiver half.
        Ok(rx)
    }

    async fn increase(&mut self, by: u32) -> Result<(), IncreaseError> {
        // Perform the addition if it does not overflow the counter.
        match self.value.checked_add(by) {
            Some(new_value) => self.value = new_value,
            None => return Err(IncreaseError::Overflow { current_value: self.value }),
        }

        // Notify all watchers and keep only the ones that are not disconnected.
        let value = self.value;
        self.watchers.retain(|watch| !watch.send(value).into_disconnected().unwrap());

        Ok(())
    }

    async fn count_to_value(
        &self, step: u32, delay: Duration,
    ) -> Result<rch::mpsc::Receiver<u32>, rtc::CallError> {
        // Create mpsc channel for counting.
        let (tx, rx) = rch::mpsc::channel(1);

        // Spawn a task to perform the counting.
        let value = self.value;
        tokio::spawn(async move {
            // Counting loop.
            for i in (0..value).step_by(step as usize) {
                // Send the value.
                if tx.send(i).await.into_disconnected().unwrap() {
                    // Abort the counting if the client dropped the
                    // receive half or disconnected.
                    break;
                }

                // Wait the specified delay.
                sleep(delay).await;
            }
        });

        // Return the receive half of the counting channel.
        Ok(rx)
    }
}


/// Server object for the counting service, keeping the state.
#[derive(Default)]
pub struct TestCommandObj {
    /// The current value.
    value: u32,
    /// The subscribed watchers.
    watchers: Vec<rch::watch::Sender<u32>>,
}

/// Implementation of remote counting service.
#[rtc::async_trait]
impl TestCommand for TestCommandObj {
    async fn value(&self) -> Result<u32, rtc::CallError> {
        Ok(self.value)
    }

    async fn watch(&mut self) -> Result<rch::watch::Receiver<u32>, rtc::CallError> {
        // Create watch channel.
        let (tx, rx) = rch::watch::channel(self.value);
        // Keep the sender half in the watchers vector.
        self.watchers.push(tx);
        // And return the receiver half.
        Ok(rx)
    }

    async fn get_address(&mut self, by: u32) -> Result<(), IncreaseError> {
        // Perform the addition if it does not overflow the counter.
        match self.value.checked_add(by) {
            Some(new_value) => self.value = new_value,
            None => return Err(IncreaseError::Overflow { current_value: self.value }),
        }

        // Notify all watchers and keep only the ones that are not disconnected.
        let value = self.value;
        self.watchers.retain(|watch| !watch.send(value).into_disconnected().unwrap());

        Ok(())
    }
}


#[tokio::main]
async fn main() {
    // Initialize logging.
    tracing_subscriber::FmtSubscriber::builder().init();

    let opt = Opt::from_args();

    println!("{:?}", opt);

    let debug_mode = opt.debug;    
    if debug_mode
    {
        println!("{:?}", opt);
    }
    let my_address = opt.bluetooth_address;
 
    let session = bluer::Session::new().await.unwrap();
  
    let adapter_names = session.adapter_names().await.unwrap();
    let adapter_name = adapter_names.first().expect("No Bluetooth adapter present");
    let mut adapter = session.adapter(adapter_name).unwrap();
    for adapter_name in adapter_names {
        println!("Checking Bluetooth adapter {}:", &adapter_name);
        let adapter_tmp = session.adapter(&adapter_name).unwrap();
        let address = adapter_tmp.address().await.unwrap();
        if  address.to_string() == my_address {
            adapter =  adapter_tmp;
            break;
        }
    };

    //let adapter_name = adapter_names.first().expect("No Bluetooth adapter present");
    //let adapter = session.adapter(adapter_name)?;
    let adapter_name = adapter.name();    
    adapter.set_powered(true).await.unwrap();
    if debug_mode
    {
        println!("Advertising on Bluetooth adapter {}", &adapter_name);
        println!("    Address:                    {}", adapter.address().await.unwrap());
        println!("    Address type:               {}", adapter.address_type().await.unwrap());
        println!("    Friendly name:              {}", adapter.alias().await.unwrap());
        println!("    System name:                {}", adapter.system_name().await.unwrap());
        println!("    Modalias:                   {:?}", adapter.modalias().await.unwrap());
        println!("    Powered:                    {:?}", adapter.is_powered().await.unwrap());        
    }

    // Create a counter object that will be shared between all clients.
    // You could also create one counter object per connection.
    let counter_obj = Arc::new(RwLock::new(CounterObj::default()));

    // Listen to TCP connections using Tokio.
    // In reality you would probably use TLS or WebSockets over HTTPS.
    println!("Listening on port {}. Press Ctrl+C to exit.", TCP_PORT);
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, TCP_PORT)).await.unwrap();

    loop {
        // Accept an incoming TCP connection.
        let (socket, addr) = listener.accept().await.unwrap();
        let (socket_rx, socket_tx) = socket.into_split();
        println!("Accepted connection from {}", addr);

        // Create a new shared reference to the counter object.
        let counter_obj = counter_obj.clone();

        // Spawn a task for each incoming connection.
        tokio::spawn(async move {
            // Create a server proxy and client for the accepted connection.
            //
            // The server proxy executes all incoming method calls on the shared counter_obj
            // with a request queue length of 1.
            //
            // Current limitations of the Rust compiler require that we explicitly
            // specify the codec.
            let (server, client) = CounterServerSharedMut::<_, codec::Default>::new(counter_obj, 1);

            // Establish a Remoc connection with default configuration over the TCP connection and
            // provide (i.e. send) the counter client to the client.
            remoc::Connect::io(remoc::Cfg::default(), socket_rx, socket_tx).provide(client).await.unwrap();

            // Serve incoming requests from the client on this task.
            // `true` indicates that requests are handled in parallel.
            server.serve(true).await;
        });
    }
}