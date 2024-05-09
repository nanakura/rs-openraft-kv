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

use openraft::Config;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::info;

use crate::app::App;
use crate::network::raft::Raft;
use crate::network::{api, management, Network};
use crate::store::new_storage;
use crate::store::Request;
use crate::store::Response;

pub mod app;
pub mod client;
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
    leader_http_addr: Option<String>,
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

    let kvs = state_machine_store.data.kvs.clone();

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

    let mut set = BTreeSet::new();
    set.insert(node_id);
    let app = App {
        id: node_id,
        api_addr: http_addr.clone(),
        rpc_addr: rpc_addr.clone(),
        raft,
        key_values: kvs,
        config,
        nodes: Arc::new(Mutex::new(set)),
    };

    let addr: SocketAddr = rpc_addr.parse().unwrap();
    let raft_node = Raft::new(Arc::new(app.clone()));
    tokio::spawn(async move {
        let addr = volo::net::Address::from(addr);

        info!("websocket server");
        volo_gen::rpc::raft::RaftServiceServer::new(raft_node)
            .run(addr)
            .await
            .unwrap();
    });

    // Create an application that will store all the instances created above, this will
    // be later used on the actix-web services.

    let server_start = web::HttpServer::new(move || {
        info!("web server");
        let app = app.clone();
        web::App::new()
            .state(app)
            .wrap(middleware::Logger::default())
            .configure(api::rest)
            .configure(management::rest())
    })
    .bind(&http_addr)
    .unwrap()
    .run();

    sleep(Duration::from_secs(1)).await;
    let client = reqwest::Client::new();
    if let Some(addr) = leader_http_addr {
        let response = client
            .post(format!("http://{}/cluster/add-learner", addr))
            .body(format!(
                "[{}, \"{}\", \"{}\"]",
                node_id, http_addr, rpc_addr
            ))
            .send()
            .await
            .unwrap();
        info!("cluster add learner resp status {}", response.status());
        let response = client
            .post(format!("http://{}/cluster/change-membership", addr))
            .send()
            .await
            .unwrap();
        info!(
            "cluster change membership resp status {}",
            response.status()
        );
    } else {
        let response = client
            .post(format!("http://{}/cluster/init", http_addr))
            .body("{}")
            .send()
            .await
            .unwrap();
        info!("cluster init resp status {}", response.status());
    }
    server_start.await?;
    Ok(())
}
