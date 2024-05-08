use std::sync::Arc;

use crate::app::App;

/// Raft protocol service.
pub struct Raft {
    app: Arc<App>,
}

impl Raft {
    pub fn new(app: Arc<App>) -> Self {
        Self { app }
    }
}

impl volo_gen::rpc::raft::RaftService for Raft {
    async fn vote(
        &self,
        req: volo_gen::rpc::raft::RaftRequest,
    ) -> Result<volo_gen::rpc::raft::RaftReply, volo_thrift::ServerError> {
        let data = req.data.as_bytes();
        let vote = serde_json::from_slice(data).unwrap();
        let resp = self.app.raft.vote(vote).await.unwrap();
        let result = serde_json::to_string(&resp).unwrap();
        Ok(volo_gen::rpc::raft::RaftReply {
            data: result.parse().unwrap(),
            error: Default::default(),
        })
    }
    async fn append(
        &self,
        req: volo_gen::rpc::raft::RaftRequest,
    ) -> Result<volo_gen::rpc::raft::RaftReply, volo_thrift::ServerError> {
        tracing::debug!("handle append");
        let data = req.data.as_bytes();
        let req = serde_json::from_slice(data).unwrap();
        let resp = self.app.raft.append_entries(req).await.unwrap();
        let result = serde_json::to_string(&resp).unwrap();
        Ok(volo_gen::rpc::raft::RaftReply {
            data: result.parse().unwrap(),
            error: Default::default(),
        })
    }
    async fn snapshot(
        &self,
        req: volo_gen::rpc::raft::RaftRequest,
    ) -> Result<volo_gen::rpc::raft::RaftReply, volo_thrift::ServerError> {
        let data = req.data.as_bytes();
        let req = serde_json::from_slice(data).unwrap();
        let resp = self.app.raft.install_snapshot(req).await.unwrap();
        let result = serde_json::to_string(&resp).unwrap();
        Ok(volo_gen::rpc::raft::RaftReply {
            data: result.parse().unwrap(),
            error: Default::default(),
        })
    }
}
