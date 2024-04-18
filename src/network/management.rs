use std::collections::BTreeMap;
use std::collections::BTreeSet;
use anyhow::Context;
use ntex::util::BytesMut;
use ntex::web;
use ntex::web::HttpResponse;
use ntex::web::types::Payload;

use openraft::error::Infallible;
use openraft::RaftMetrics;

use crate::app::App;
use crate::network::err::HandlerResponse;
use crate::Node;
use crate::NodeId;

// --- Cluster management

pub fn rest(cfg: &mut web::ServiceConfig) {
    cfg.route("/add-learner", web::post().to(add_learner))
        .route("/change-membership", web::post().to(change_membership))
        .route("/init", web::post().to(init))
        .route("/metrics", web::get().to(metrics));
}

/// Add a node as **Learner**.
///
/// A Learner receives log replication from the leader but does not vote.
/// This should be done before adding a node as a member into the cluster
/// (by calling `change-membership`)
async fn add_learner(mut payload: Payload, mut state: web::types::State<App>) -> HandlerResponse {
    let mut bytes = BytesMut::new();
    while let Some(item) = ntex::util::stream_recv(&mut payload).await {
        bytes.extend_from_slice(&item.unwrap());
    }
    let (node_id, api_addr, rpc_addr): (NodeId, String, String) = serde_json::from_slice(&bytes.to_vec()[..]).context("deserialize json failed")?;
    let node = Node { rpc_addr, api_addr };
    let res = state.raft.add_learner(node_id, node, true).await;
    Ok(HttpResponse::Ok().json(&res))
}

/// Changes specified learners to members, or remove members.
async fn change_membership(mut payload: Payload, mut state: web::types::State<App>) -> HandlerResponse {
    let mut bytes = BytesMut::new();
    while let Some(item) = ntex::util::stream_recv(&mut payload).await {
        bytes.extend_from_slice(&item.unwrap());
    }
    let body: BTreeSet<NodeId> = serde_json::from_slice(&bytes.to_vec()[..]).context("deserialize json failed")?;
    let res = state.raft.change_membership(body, false).await;
    Ok(HttpResponse::Ok().json(&res))
}

/// Initialize a single-node cluster.
async fn init(mut state: web::types::State<App>) -> HandlerResponse {
    let mut nodes = BTreeMap::new();
    let node = Node {
        api_addr: state.api_addr.clone(),
        rpc_addr: state.rpc_addr.clone(),
    };

    nodes.insert(state.id, node);
    let res = state.raft.initialize(nodes).await;
    Ok(HttpResponse::Ok().json(&res))
}

/// Get the latest metrics of the cluster
async fn metrics(mut state: web::types::State<App>) -> HandlerResponse {
    let metrics = state.raft.metrics().borrow().clone();

    let res: Result<RaftMetrics<NodeId, Node>, Infallible> = Ok(metrics);
    Ok(HttpResponse::Ok().json(&res))
}
