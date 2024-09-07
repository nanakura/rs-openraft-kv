use crate::NodeId;
use crossbeam_channel::{unbounded, Receiver, SendError, Sender};
use parking_lot::Mutex;
use std::sync::OnceLock;
use tracing::info;

static MESSAGE_QUEUE: OnceLock<Mutex<(Sender<MqTask>, Receiver<MqTask>)>> = OnceLock::new();

fn get_message_queue() -> &'static Mutex<(Sender<MqTask>, Receiver<MqTask>)> {
    MESSAGE_QUEUE.get_or_init(|| Mutex::new(unbounded::<MqTask>()))
}

#[derive(Debug, Clone)]
pub enum MqTask {
    InitNodeReq {
        node_id: NodeId,
        http_addr: String,
        rpc_addr: String,
        leader_http_addr: Option<String>,
    },
}

pub fn send_task(task: MqTask) -> Result<(), SendError<MqTask>> {
    get_message_queue().lock().0.send(task)
}

#[tokio::main]
pub async fn mq_handler() {
    loop {
        if let Ok(task) = get_message_queue().lock().1.recv() {
            match task {
                MqTask::InitNodeReq {
                    node_id,
                    http_addr,
                    rpc_addr,
                    leader_http_addr,
                } => {
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
                }
            }
        }
    }
}
