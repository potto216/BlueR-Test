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
use tracing_subscriber::{layer::SubscriberExt, Registry};
use std::io::Stderr;
use tracing_stackdriver::Stackdriver;

/// BlueR testing tool.
#[derive(Parser)]
struct Opts {
    /// Show additional information for troubleshooting such as details about the adapters.
    #[clap(short, long)]
    debug: bool,
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

#[tokio::main]
async fn main() -> Result<()> {

  //  tracing_subscriber::FmtSubscriber::builder().init();

    let make_writer = || std::io::Stderr;
    let stackdriver = Stackdriver::with_writer(make_writer); // writes to std::io::Stderr
    let subscriber = Registry::default().with(stackdriver);

    tracing::subscriber::set_global_default(subscriber).expect("Could not set up global logger");

    log::info!("starting up");


    let opt = Opts::parse();

    let debug = opt.debug;
    let port = opt.port;
    match opt.cmd {
        Command::Server => run_server(debug, port).await,
        Command::Client(opts) => run_client(debug, port, opts).await,
    }
}
