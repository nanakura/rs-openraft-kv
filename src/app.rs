use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use openraft::Config;
use tokio::sync::Mutex;

use crate::ExampleRaft;
use crate::NodeId;

pub static RAFT_CLIENT: OnceLock<ExampleRaft> = OnceLock::new();

// Representation of an application state. This struct can be shared around to share
// instances of raft, store and more.
#[derive(Clone)]
pub struct RaftState {
    pub id: NodeId,
    pub api_addr: String,
    pub rpc_addr: String,
    pub raft: ExampleRaft,
    // pub key_values: Arc<RwLock<BTreeMap<String, String>>>,
    pub config: Arc<Config>,
    pub nodes: Arc<Mutex<BTreeSet<NodeId>>>,
}

#[derive(Clone)]
pub struct HttpServerApp {
    pub id: NodeId,
    pub api_addr: String,
    pub rpc_addr: String,
    pub nodes: Arc<Mutex<BTreeSet<NodeId>>>,
}
