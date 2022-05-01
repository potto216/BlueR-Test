//! BlueR testing tool.
//#[macro_use]
//extern crate log;
//use env_logger::Env;

mod client;
mod rpc;
mod server;

use anyhow::Result;
use clap::Parser;
use client::{run_client, ClientOpts};
use server::run_server;
//use tracing_subscriber::{layer::SubscriberExt, Registry};
//use std::io::Stderr;
//use tracing_stackdriver::Stackdriver;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};


/// BlueR testing tool.
#[derive(Parser)]
struct Opts {
    /// Show additional information for troubleshooting such as details about the adapters.
    #[clap(short, long)]
    debug_mode: bool,
    /// TCP port number for connection between client and server.
    #[clap(short, long, default_value = "8650")]
    port: u16,
    /// Command.
    #[clap(subcommand)]
    cmd: Command,
}

/// Command.
#[derive(Parser)]
enum Command {
    /// Run the server.
    Server,
    /// Connect to a server.
    Client(ClientOpts),
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Command::Server => write!(f, "Server"),
            Command::Client(_) => write!(f, "Client"),
        }        
    }
}


#[tokio::main]
async fn main() -> Result<()> {

//   tracing_subscriber::FmtSubscriber::builder().init();

    //let make_writer = || std::io::Stderr;
    //let stackdriver = Stackdriver::with_writer(make_writer); // writes to std::io::Stderr
  //  let subscriber = Registry::default();
//    let subscriber = tracing_subscriber::fmt()
 //   .with_writer(std::io::stderr)
  //  .finish();
    

    //tracing::subscriber::set_global_default(subscriber).expect("Could not set up global logger");
 // We are falling back to printing all spans at info-level or above 
    // if the RUST_LOG environment variable has not been set.
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    let file_appender = tracing_appender::rolling::hourly("/home/user/log", "prefix.log");
    let formatting_layer = BunyanFormattingLayer::new(
        "BlueR-Test".into(), 
        // Output the formatted spans to stdout. 
        file_appender
    );
    //  make_write:std::io::stdout
    // The `with` method is provided by `SubscriberExt`, an extension
    // trait for `Subscriber` exposed by `tracing_subscriber`
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    // `set_global_default` can be used by applications to specify 
    // what subscriber should be used to process spans.  
    set_global_default(subscriber).expect("Failed to set subscriber");

    let opt = Opts::parse();

    let debug_mode = opt.debug_mode;
    let port = opt.port;
    let cmd = opt.cmd;
    let startup_span = tracing::info_span!(
        "Starting up with the command line",
        %port,
        %cmd 
        
    );
    let _startup_span_guard = startup_span.enter();


    match cmd {
        Command::Server => run_server(debug_mode, port).await,
        Command::Client(opts) => run_client(debug_mode, port, opts).await,
    }
}
