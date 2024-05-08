namespace rs rpc.raft


struct RaftRequest {
    1: required string data,
}

struct RaftReply {
    1: required string data,
    2: required string error
}

service RaftService {
    RaftReply Vote (1: RaftRequest req),
    RaftReply Append (1: RaftRequest req),
    RaftReply Snapshot (1: RaftRequest req),
}
