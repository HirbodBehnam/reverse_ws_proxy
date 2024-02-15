use clap::Parser;

mod arguments;
mod local;
mod remote;

#[tokio::main]
async fn main() {
    // Initialize tracing and log
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = arguments::Args::parse();

    // Start the server or client
    match args.command {
        arguments::Commands::Local {
            tcp_listen_address,
            cloudflare_listen_address,
        } => local::start_local_server(&cloudflare_listen_address, &tcp_listen_address).await,
        arguments::Commands::Server {
            cloudflare_server_address,
            forward_address,
        } => remote::start_remote_controller(cloudflare_server_address, forward_address).await,
    };
}
