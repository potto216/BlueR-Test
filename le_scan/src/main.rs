//! Scans for advertisements from a particular device  and returns  success or not
//TODO: Add better exit codes https://www.joshmcguigan.com/blog/custom-exit-status-codes-rust/, https://github.com/JoshMcguigan/exit

use bluer::{gatt::remote::Characteristic, AdapterEvent, Device, Result};
use futures::{pin_mut, StreamExt};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::sleep,
};

use structopt::StructOpt;


#[derive(Debug, StructOpt)]
#[structopt(name = "le_scan", about = "A command tool to test BLE advertisers")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short, long)]
    _debug: bool,

    /// Advertiser address
    // short and long flags (-a, --advertiser) will be deduced from the field's name     
    #[structopt(short, long)]
    advertiser: String,

    /// Scanner address
    // short and long flags (-s, --scanner) will be deduced from the field's name     
    #[structopt(short, long)]
    scanner: String,

}


include!("gatt.inc");

async fn find_our_characteristic(device: &Device, remote_target_address: &String) -> Result<Option<Characteristic>> {
    let addr = device.address();
    let uuids = device.uuids().await?.unwrap_or_default();
    println!("Discovered device {} with service UUIDs {:?}", addr, &uuids);
    if remote_target_address == &addr.to_string()
    {
        println!("****Found device of interest!****");
    }
    let md = device.manufacturer_data().await?;
    println!("    Manufacturer data: {:x?}", &md);

    if uuids.contains(&SERVICE_UUID) {
        println!("    Device provides our service!");

        sleep(Duration::from_secs(2)).await;
        if !device.is_connected().await? {
            println!("    Connecting...");
            let mut retries = 2;
            loop {
                match device.connect().await {
                    Ok(()) => break,
                    Err(err) if retries > 0 => {
                        println!("    Connect error: {}", &err);
                        retries -= 1;
                    }
                    Err(err) => return Err(err),
                }
            }
            println!("    Connected");
        } else {
            println!("    Already connected");
        }

        println!("    Enumerating services...");
        for service in device.services().await? {
            let uuid = service.uuid().await?;
            println!("    Service UUID: {}", &uuid);
            if uuid == SERVICE_UUID {
                println!("    Found our service!");
                for char in service.characteristics().await? {
                    let uuid = char.uuid().await?;
                    println!("    Characteristic UUID: {}", &uuid);
                    if uuid == CHARACTERISTIC_UUID {
                        println!("    Found our characteristic!");
                        return Ok(Some(char));
                    }
                }
            }
        }

        println!("    Not found!");
    }

    Ok(None)
}

async fn exercise_characteristic(char: &Characteristic) -> Result<()> {
    println!("    Characteristic flags: {:?}", char.flags().await?);
    sleep(Duration::from_secs(1)).await;

    if char.flags().await?.read {
        println!("    Reading characteristic value");
        let value = char.read().await?;
        println!("    Read value: {:x?}", &value);
        sleep(Duration::from_secs(1)).await;
    }

    let data = vec![0xee, 0x11, 0x11, 0x0];
    println!("    Writing characteristic value {:x?} using function call", &data);
    char.write(&data).await?;
    sleep(Duration::from_secs(1)).await;

    if char.flags().await?.read {
        let value = char.read().await?;
        println!("    Read value back: {:x?}", &value);
        sleep(Duration::from_secs(1)).await;
    }

    println!("    Obtaining write IO");
    let mut write_io = char.write_io().await?;
    println!("    Write IO obtained");
    println!("    Writing characteristic value {:x?} five times using IO", &data);
    for _ in 0..5u8 {
        let written = write_io.write(&data).await?;
        println!("    {} bytes written", written);
    }
    println!("    Closing write IO");
    drop(write_io);
    sleep(Duration::from_secs(1)).await;

    println!("    Starting notification session");
    {
        let notify = char.notify().await?;
        pin_mut!(notify);
        for _ in 0..5u8 {
            match notify.next().await {
                Some(value) => {
                    println!("    Notification value: {:x?}", &value);
                }
                None => {
                    println!("    Notification session was terminated");
                }
            }
        }
        println!("    Stopping notification session");
    }
    sleep(Duration::from_secs(1)).await;

    println!("    Obtaining notification IO");
    let mut notify_io = char.notify_io().await?;
    println!("    Obtained notification IO with MTU={}", notify_io.mtu());
    for _ in 0..5u8 {
        let mut buf = vec![0; notify_io.mtu()];
        match notify_io.read(&mut buf).await {
            Ok(0) => {
                println!("    Notification IO end of stream");
                break;
            }
            Ok(read) => {
                println!("    Notified with {} bytes: {:x?}", read, &buf[0..read]);
            }
            Err(err) => {
                println!("    Notification IO failed: {}", &err);
                break;
            }
        }
    }
    println!("    Stopping notification IO");
    drop(notify_io);
    sleep(Duration::from_secs(1)).await;

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    let opt = Opt::from_args();
    env_logger::init();

    println!("{:?}", opt);

    let my_address = opt.scanner;
    let remote_target_address = opt.advertiser;

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

    println!("    Adapter name:               {}", adapter_name);
    println!("    Address:                    {}", adapter.address().await?);
    println!("    Address type:               {}", adapter.address_type().await?);
    println!("    Friendly name:              {}", adapter.alias().await?);
    println!("    System name:                {}", adapter.system_name().await?);
    println!("    Modalias:                   {:?}", adapter.modalias().await?);
    println!("    Powered:                    {:?}", adapter.is_powered().await?);        

    {
        let discover = adapter.discover_devices().await?;
        pin_mut!(discover);
        while let Some(evt) = discover.next().await {
            match evt {
                AdapterEvent::DeviceAdded(addr) => {
                    let device = adapter.device(addr)?;

                    let addr = device.address();
                    let uuids = device.uuids().await?.unwrap_or_default();
                    println!("Discovered device {} with service UUIDs {:?}", addr, &uuids);
                    if remote_target_address == addr.to_string()
                    {
                        println!("Result: Found");
                        break;                        
                    }
                    

                }
                AdapterEvent::DeviceRemoved(addr) => {
                    println!("Device removed {}", addr);
                }
                _ => (),
            }

        }
        println!("Stopping discovery");
    }

    sleep(Duration::from_secs(1)).await;
    Ok(())
}
