#![allow(clippy::uninlined_format_args)]
#![deny(unused_qualifications)]

use ntex::web;
use ntex::web::middleware;
use std::collections::BTreeSet;
use std::fmt::Display;
use std::io::Cursor;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use ntex::time::sleep;

use openraft::Config;
use tokio::sync::Mutex;
use tracing::info;

use crate::app::{HttpServerApp, RaftState, RAFT_CLIENT};
pub use crate::mq::mq_handler;
use crate::mq::send_task;
use crate::mq::MqTask::InitNodeReq;
use crate::network::raft::Raft;
use crate::network::{api, management, Network};
use crate::store::new_storage;
use crate::store::Request;
use crate::store::Response;

pub mod app;
pub mod client;
mod mq;
pub mod network;
pub mod store;

pub type NodeId = u64;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
pub struct Node {
    pub rpc_addr: String,
    pub api_addr: String,
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Node {{ rpc_addr: {}, api_addr: {} }}",
            self.rpc_addr, self.api_addr
        )
    }
}

pub type SnapshotData = Cursor<Vec<u8>>;

openraft::declare_raft_types!(
    pub TypeConfig:
        D = Request,
        R = Response,
        Node = Node,
);

pub mod typ {
    use openraft::error::Infallible;

    use crate::Node;
    use crate::NodeId;
    use crate::TypeConfig;

    pub type Entry = openraft::Entry<TypeConfig>;

    pub type RaftError<E = Infallible> = openraft::error::RaftError<NodeId, E>;
    pub type RPCError<E = Infallible> = openraft::error::RPCError<NodeId, Node, RaftError<E>>;

    pub type ClientWriteError = openraft::error::ClientWriteError<NodeId, Node>;
    pub type CheckIsLeaderError = openraft::error::CheckIsLeaderError<NodeId, Node>;
    pub type ForwardToLeader = openraft::error::ForwardToLeader<NodeId, Node>;
    pub type InitializeError = openraft::error::InitializeError<NodeId, Node>;

    pub type ClientWriteResponse = openraft::raft::ClientWriteResponse<TypeConfig>;
}

pub type ExampleRaft = openraft::Raft<TypeConfig>;

pub async fn start_example_raft_node<P>(
    node_id: NodeId,
    dir: P,
    http_addr: String,
    rpc_addr: String,
) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    // Create a configuration for the raft instance.
    let config = Config {
        heartbeat_interval: 250,
        election_timeout_min: 299,
        ..Default::default()
    };

    let config = Arc::new(config.validate().unwrap());

    let (log_store, state_machine_store) = new_storage(&dir).await;

    // Create the network layer that will connect and communicate the raft instances and
    // will be used in conjunction with the store created above.
    let network = Network {};

    // Create a local raft instance.
    let raft = openraft::Raft::new(
        node_id,
        config.clone(),
        network,
        log_store,
        state_machine_store,
    )
    .await
    .unwrap();
    RAFT_CLIENT.get_or_init(|| raft.clone());

    let mut set = BTreeSet::new();
    set.insert(node_id);
    let app = RaftState {
        id: node_id,
        api_addr: http_addr.clone(),
        rpc_addr: rpc_addr.clone(),
        raft,
        config,
        nodes: Arc::new(Mutex::new(set)),
    };

    let addr: SocketAddr = rpc_addr.parse().unwrap();
    let raft_node = Raft::new(Arc::new(app.clone()));
    let addr = volo::net::Address::from(addr);

    info!("websocket server");
    volo_gen::rpc::raft::RaftServiceServer::new(raft_node)
        .run(addr)
        .await
        .unwrap();
    Ok(())
}

pub async fn start_ntex(
    node_id: NodeId,
    http_addr: String,
    rpc_addr: String,
    leader_http_addr: Option<String>,
) -> std::io::Result<()> {
    // Create an application that will store all the instances created above, this will
    // be later used on the ntex services.
    let mut set = BTreeSet::new();
    set.insert(node_id);
    let app = HttpServerApp {
        id: node_id,
        api_addr: http_addr.clone(),
        rpc_addr: rpc_addr.clone(),
        nodes: Arc::new(Mutex::new(set)),
    };
    let server_start = web::HttpServer::new(move || {
        info!("web server");
        web::App::new()
            .state(app.clone())
            .wrap(middleware::Logger::default())
            .configure(api::rest)
            .configure(management::rest())
    })
    .bind(&http_addr)
    .unwrap()
    .run();

    sleep(Duration::from_secs(1)).await;
    let _ = send_task(InitNodeReq {
        node_id,
        http_addr,
        rpc_addr,
        leader_http_addr,
    });
    server_start.await?;
    Ok(())
}
