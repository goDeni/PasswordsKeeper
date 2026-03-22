use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use sec_store_server::{serve, ServerConfigPaths};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "sec_store_server")]
struct Cli {
    #[arg(long, default_value = "127.0.0.1:8443")]
    bind_addr: SocketAddr,
    #[arg(long)]
    data_dir: PathBuf,
    #[arg(long)]
    server_cert_pem: PathBuf,
    #[arg(long)]
    server_key_pem: PathBuf,
    #[arg(long)]
    client_ca_cert_pem: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    serve(ServerConfigPaths {
        bind_addr: cli.bind_addr,
        data_dir: cli.data_dir,
        server_cert_pem: cli.server_cert_pem,
        server_key_pem: cli.server_key_pem,
        client_ca_cert_pem: cli.client_ca_cert_pem,
    })
    .await
}
