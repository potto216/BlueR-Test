//! BlueR testing tool.

mod client;
mod rpc;
mod server;

use anyhow::Result;
use clap::Parser;
use client::{run_client, ClientOpts};
use server::run_server;

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
    tracing_subscriber::FmtSubscriber::builder().init();
    let opt = Opts::parse();

    let debug = opt.debug;
    let port = opt.port;
    match opt.cmd {
        Command::Server => run_server(debug, port).await,
        Command::Client(opts) => run_client(debug, port, opts).await,
    }
}
