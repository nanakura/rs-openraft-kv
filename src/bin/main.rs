use clap::Parser;
use raft_demo3::{mq_handler, start_example_raft_node, start_ntex};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Opt {
    #[clap(long)]
    pub id: u64,

    #[clap(long)]
    pub http_addr: String,

    #[clap(long)]
    pub rpc_addr: String,

    #[clap(long)]
    pub leader_http_addr: Option<String>,
}

#[ntex::main]
async fn start_http_server(options: Opt) -> std::io::Result<()> {
    start_ntex(
        options.id,
        format!("{}-db", options.id),
        options.http_addr,
        options.leader_http_addr,
    )
    .await
}

#[tokio::main]
async fn start_raft_server(options: Opt) -> std::io::Result<()> {
    start_example_raft_node(
        options.id,
        format!("{}-db", options.id),
        options.http_addr,
        options.rpc_addr,
    )
    .await
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup the logger
    tracing_subscriber::fmt()
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true)
        .with_ansi(false)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let options = Opt::parse();
    let options2 = options.clone();
    let t1 = std::thread::spawn(move || start_raft_server(options.clone()));
    let t2 = std::thread::spawn(|| mq_handler());
    let t3 = std::thread::spawn(move || start_http_server(options2.clone()));
    let _ = t1.join().unwrap();
    let _ = t2.join().unwrap();
    let _ = t3.join().unwrap();
    Ok(())
}
