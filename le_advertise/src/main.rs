//! Perform a Bluetooth LE advertisement.

use bluer::adv::Advertisement;
use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    time::sleep,
};

use structopt::StructOpt;
//use std::str::FromStr;


#[derive(Debug, StructOpt)]
#[structopt(name = "le_advertise", about = "A command tool to generate BLE advertisements")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short, long)]
    _debug: bool,

    /// Advertiser address
    // short and long flags (-a, --advertiser) will be deduced from the field's name     
    #[structopt(short, long)]
    advertiser: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {

    let opt = Opt::from_args();
    env_logger::init();

    println!("{:?}", opt);

    let my_address = opt.advertiser;

    let session = bluer::Session::new().await?;
            
        
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

    println!("    Address:                    {}", adapter.address().await?);
    println!("    Address type:               {}", adapter.address_type().await?);
    println!("    Friendly name:              {}", adapter.alias().await?);
    println!("    System name:                {}", adapter.system_name().await?);
    println!("    Modalias:                   {:?}", adapter.modalias().await?);
    println!("    Powered:                    {:?}", adapter.is_powered().await?);    

    println!("Advertising on Bluetooth adapter {} with address {}", &adapter_name, adapter.address().await?);
    let le_advertisement = Advertisement {
        advertisement_type: bluer::adv::Type::Peripheral,
        service_uuids: vec!["123e4567-e89b-12d3-a456-426614174000".parse().unwrap()].into_iter().collect(),
        discoverable: Some(true),
        local_name: Some("le_advertise".to_string()),
        ..Default::default()
    };
    println!("{:?}", &le_advertisement);
    let handle = adapter.advertise(le_advertisement).await?;

    println!("Press enter to quit");
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    let _ = lines.next_line().await;

    println!("Removing advertisement");
    drop(handle);
    sleep(Duration::from_secs(1)).await;

    Ok(())
}