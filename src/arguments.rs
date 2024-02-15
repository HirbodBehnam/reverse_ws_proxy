use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about)]
#[command(about = "A websocket based reverse proxy", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Run as the server that cloudflare connects to", long_about = None)]
    Local {
        #[arg(short = 'l', long, help = "On what address we should listen and accept TCP connections?")]
        tcp_listen_address: String,
        #[arg(short = 'c', long, help = "On what address we should listen and accept the connections from Cloudflare?")]
        cloudflare_listen_address: String,
    },
    #[command(about = "Run as the program that connects to cloudflare", long_about = None)]
    Server {
        #[arg(short = 'c', long, help = "What is the address of cloudflare that we should send the websockets to?")]
        cloudflare_server_address: String,
        #[arg(short = 'f', long, help = "Where we should forward the websocket traffic?")]
        forward_address: String,
    }
}