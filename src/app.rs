use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use openraft::Config;
use tokio::sync::{Mutex, RwLock};

use crate::ExampleRaft;
use crate::NodeId;

// Representation of an application state. This struct can be shared around to share
// instances of raft, store and more.
#[derive(Clone)]
pub struct App {
    pub id: NodeId,
    pub api_addr: String,
    pub rpc_addr: String,
    pub raft: ExampleRaft,
    pub key_values: Arc<RwLock<BTreeMap<String, String>>>,
    pub config: Arc<Config>,
    pub nodes: Arc<Mutex<BTreeSet<NodeId>>>,
}
