//! Scans for advertisements from a particular device  and returns  success or not
//TODO: Add better exit codes https://www.joshmcguigan.com/blog/custom-exit-status-codes-rust/, https://github.com/JoshMcguigan/exit

use bluer::{AdapterEvent,  Uuid};
use futures::{pin_mut, StreamExt};
use std::time::Duration;
use tokio::{
    time::sleep,
};

use structopt::StructOpt;


#[derive(Debug, StructOpt)]
#[structopt(name = "le_scan", about = "A command tool to test BLE advertisers")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short, long, help="Show additional information for troubleshooting such as details about the adapters")]
    debug: bool,

    /// Scanner address
    // short and long flags (-s, --scanner) will be deduced from the field's name     
    #[structopt(short, long, required=true, help="The scanner address in the form XX:XX:XX:XX:XX:XX  ex: 5C:F3:70:7B:F5:66")]
    scanner: String,

    // short and long flags (-a, --advertiser) will be deduced from the field's name     
    #[structopt(short, long, required=true, help="The advertisement address in the form XX:XX:XX:XX:XX:XX  ex: 5C:F3:70:A1:71:0F")]
    advertiser: String,

    // short and long flags (-u, --uuid-service) will be deduced from the field's name     
    #[structopt(short, long, default_value="", help="This is the service to except from the advertiser. ex: 123e4567-e89b-12d3-a456-426614174000")]
    uuid_service: String,

}




#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    let opt = Opt::from_args();
    
    env_logger::init();

    let debug_mode = opt.debug;    
    if debug_mode
    {
        println!("{:?}", opt);
    }

    let my_address = opt.scanner;
    let remote_target_address = opt.advertiser;

    let session = bluer::Session::new().await?;

    let uuid_service = opt.uuid_service;         
        
    let adapter_names = session.adapter_names().await?;
    let adapter_name = adapter_names.first().expect("No Bluetooth adapter present");
    let mut adapter = session.adapter(adapter_name)?;
    for adapter_name in adapter_names {
        println!("Checking Bluetooth adapter {}:", &adapter_name);
        let adapter_tmp = session.adapter(&adapter_name)?;
        let address = adapter_tmp.address().await?;
        if  address.to_string() == my_address {
            adapter =  adapter_tmp;
            break;
        }
    };
    //let adapter_name = adapter_names.first().expect("No Bluetooth adapter present");
    //let adapter = session.adapter(adapter_name)?;
    let adapter_name = adapter.name();
    adapter.set_powered(true).await?;

    if debug_mode
    {
        println!("    Adapter name:               {}", adapter_name);
        println!("    Address:                    {}", adapter.address().await?);
        println!("    Address type:               {}", adapter.address_type().await?);
        println!("    Friendly name:              {}", adapter.alias().await?);
        println!("    System name:                {}", adapter.system_name().await?);
        println!("    Modalias:                   {:?}", adapter.modalias().await?);
        println!("    Powered:                    {:?}", adapter.is_powered().await?);        
    }
    {
        let discover = adapter.discover_devices().await?;
        pin_mut!(discover);
        while let Some(evt) = discover.next().await {
            match evt {
                AdapterEvent::DeviceAdded(addr) => {
                    let device = adapter.device(addr)?;

                    let addr = device.address();
                    let uuids = device.uuids().await?.unwrap_or_default();


                    if debug_mode
                    {                
                        println!("Discovered device {} with service UUIDs {:?}", addr, &uuids);
                    }
                    if remote_target_address == addr.to_string()
                    {
                        if uuid_service  != ""
                        {
                            let uuid_search_for = Uuid::parse_str(&uuid_service).unwrap();
                            if uuids.contains(&uuid_search_for)
                            {
                                println!("Result: Found {} with uuid {}", addr, uuid_search_for);
                            }
                            else
                            {
                                println!("Result: Found {}, but not required uuid {}", addr, uuid_search_for);
                            }
                            
                            break;   
                        }
                        else                        
                        {
                            println!("Result: Found {}", addr);
                            break;    
                        }

                     
                    }
                    

                }
                AdapterEvent::DeviceRemoved(addr) => {
                    if debug_mode
                    {                
                        println!("Device removed {}", addr);
                    }
                }
                _ => (),
            }

        }
        if debug_mode
        {
            println!("Stopping discovery");
        }
        
    }

    sleep(Duration::from_secs(1)).await;
    Ok(())
}
