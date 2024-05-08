use std::fmt::Display;
use std::net::SocketAddr;

use openraft::error::InstallSnapshotError;
use openraft::error::NetworkError;
use openraft::error::RPCError;
use openraft::error::RaftError;
use openraft::network::RPCOption;
use openraft::network::RaftNetwork;
use openraft::network::RaftNetworkFactory;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::InstallSnapshotRequest;
use openraft::raft::InstallSnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;
use serde::de::DeserializeOwned;
use volo_gen::rpc::raft::RaftRequest;
use volo_thrift::ClientError;

use crate::Node;
use crate::NodeId;
use crate::TypeConfig;

pub struct Network {}

// NOTE: This could be implemented also on `Arc<ExampleNetwork>`, but since it's empty, implemented
// directly.
impl RaftNetworkFactory<TypeConfig> for Network {
    type Network = NetworkConnection;

    #[tracing::instrument(level = "debug", skip_all)]
    async fn new_client(&mut self, target: NodeId, node: &Node) -> Self::Network {
        let addr: SocketAddr = node.rpc_addr.parse().unwrap();
        let saddr = format!("ws://{}", node.rpc_addr);

        let client = volo_gen::rpc::raft::RaftServiceClientBuilder::new("raft-service")
            .address(addr)
            .build();

        NetworkConnection {
            addr: saddr,
            client,
            target,
        }
    }
}

pub struct NetworkConnection {
    addr: String,
    client: volo_gen::rpc::raft::RaftServiceClient,
    target: NodeId,
}

impl NetworkConnection {
    async fn c<E: std::error::Error + DeserializeOwned>(
        &mut self,
    ) -> Result<&volo_gen::rpc::raft::RaftServiceClient, RPCError<NodeId, Node, E>> {
        Ok(&self.client)
        // .ok_or_else(|| RPCError::Network(NetworkError::from(AnyError::default())))
    }
}

#[derive(Debug)]
struct ErrWrap(Box<dyn std::error::Error>);

impl Display for ErrWrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for ErrWrap {}

fn to_error<E: std::error::Error + 'static + Clone>(
    e: ClientError,
    target: NodeId,
) -> RPCError<NodeId, Node, E> {
    RPCError::Network(NetworkError::new(&e))
    // match e {
    //     toy_rpc::Error::IoError(e) => RPCError::Network(NetworkError::new(&e)),
    //     toy_rpc::Error::ParseError(e) => RPCError::Network(NetworkError::new(&ErrWrap(e))),
    //     toy_rpc::Error::Internal(e) => {
    //         let any: &dyn Any = &e;
    //         let error: &E = any.downcast_ref().unwrap();
    //         RPCError::RemoteError(RemoteError::new(target, error.clone()))
    //     }
    //     e @ (toy_rpc::Error::InvalidArgument
    //     | toy_rpc::Error::ServiceNotFound
    //     | toy_rpc::Error::MethodNotFound
    //     | toy_rpc::Error::ExecutionError(_)
    //     | toy_rpc::Error::Canceled(_)
    //     | toy_rpc::Error::Timeout(_)
    //     | toy_rpc::Error::MaxRetriesReached(_)) => RPCError::Network(NetworkError::new(&e)),
    // }
}

// With nightly-2023-12-20, and `err(Debug)` in the instrument macro, this gives the following lint
// warning. Without `err(Debug)` it is OK. Suppress it with `#[allow(clippy::blocks_in_conditions)]`
//
// warning: in a `match` scrutinee, avoid complex blocks or closures with blocks; instead, move the
// block or closure higher and bind it with a `let`
//
//    --> src/network/raft_network_impl.rs:99:91
//     |
// 99  |       ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>>
// {
//     |  ___________________________________________________________________________________________^
// 100 | |         tracing::debug!(req = debug(&req), "append_entries");
// 101 | |
// 102 | |         let c = self.c().await?;
// ...   |
// 108 | |         raft.append(req).await.map_err(|e| to_error(e, self.target))
// 109 | |     }
//     | |_____^
//     |
//     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#blocks_in_conditions
//     = note: `#[warn(clippy::blocks_in_conditions)]` on by default
#[allow(clippy::blocks_in_conditions)]
impl RaftNetwork<TypeConfig> for NetworkConnection {
    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn append_entries(
        &mut self,
        req: AppendEntriesRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        tracing::debug!(req = debug(&req), "append_entries");

        let c = self.c().await?;
        tracing::debug!("got connection");

        let req = serde_json::to_string(&req).unwrap();
        let x = c
            .append(RaftRequest {
                data: req.parse().unwrap(),
            })
            .await
            .map_err(|e| to_error(e, self.target))?;

        let resp = serde_json::from_str(x.data.as_str()).unwrap();
        Ok(resp)
    }

    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn install_snapshot(
        &mut self,
        req: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<
        InstallSnapshotResponse<NodeId>,
        RPCError<NodeId, Node, RaftError<NodeId, InstallSnapshotError>>,
    > {
        tracing::debug!(req = debug(&req), "install_snapshot");
        let req = serde_json::to_string(&req).unwrap();
        let x = self
            .c()
            .await?
            .snapshot(RaftRequest {
                data: req.parse().unwrap(),
            })
            .await
            .map_err(|e| to_error(e, self.target))?;
        let resp = serde_json::from_str(x.data.as_str()).unwrap();
        Ok(resp)
    }

    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn vote(
        &mut self,
        req: VoteRequest<NodeId>,
        _option: RPCOption,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, Node, RaftError<NodeId>>> {
        tracing::debug!(req = debug(&req), "vote");
        let req = serde_json::to_string(&req).unwrap();
        let x = self
            .c()
            .await?
            .vote(RaftRequest {
                data: req.parse().unwrap(),
            })
            .await
            .map_err(|e| to_error(e, self.target))?;
        let resp = serde_json::from_str(x.data.as_str()).unwrap();
        Ok(resp)
    }
}
