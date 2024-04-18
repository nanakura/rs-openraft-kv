use anyhow::Context;
use ntex::util::BytesMut;
use ntex::web;
use ntex::web::HttpResponse;
use ntex::web::types::Payload;

use openraft::error::CheckIsLeaderError;
use openraft::error::Infallible;

use crate::app::App;
use crate::network::err::HandlerResponse;
use crate::Node;
use crate::NodeId;



pub fn rest(cfg: &mut web::ServiceConfig) {
    cfg.route("/write", web::post().to(write))
        .route("/read", web::post().to(read))
        .route("/consistent_read", web::post().to(consistent_read));
}
/**
 * Application API
 *
 * This is where you place your application, you can use the example below to create your
 * API. The current implementation:
 *
 *  - `POST - /write` saves a value in a key and sync the nodes.
 *  - `POST - /read` attempt to find a value from a given key.
 */
async fn write(mut payload: Payload, state: web::types::State<App>) -> HandlerResponse {
    let mut bytes = BytesMut::new();
    while let Some(item) = ntex::util::stream_recv(&mut payload).await {
        bytes.extend_from_slice(&item.unwrap());
    }
    let body = serde_json::from_slice(&bytes.to_vec()[..]).context("deserialize json failed")?;
    let res = state.raft.client_write(body).await.context("client write failed")?;
    Ok(HttpResponse::Ok().json(&res))
}

async fn read(mut payload: Payload, state: web::types::State<App>) -> HandlerResponse {
    let mut bytes = BytesMut::new();
    while let Some(item) = ntex::util::stream_recv(&mut payload).await {
        bytes.extend_from_slice(&item.unwrap());
    }
    let key: String = serde_json::from_slice(&bytes.to_vec()[..]).context("deserialize json failed")?;
    let kvs = state.key_values.read().await;
    let value = kvs.get(&key);

    let res: Result<String, Infallible> = Ok(value.cloned().unwrap_or_default());
    Ok(HttpResponse::Ok().json(&res))
}

async fn consistent_read(mut payload: Payload, state: web::types::State<App>) -> HandlerResponse {
    let ret = state.raft.ensure_linearizable().await;

    match ret {
        Ok(_) => {
            let mut bytes = BytesMut::new();
            while let Some(item) = ntex::util::stream_recv(&mut payload).await {
                bytes.extend_from_slice(&item.unwrap());
            }
            let key: String = serde_json::from_slice(&bytes.to_vec()[..]).context("deserialize json failed")?;
            let kvs = state.key_values.read().await;

            let value = kvs.get(&key);

            let res: Result<String, CheckIsLeaderError<NodeId, Node>> = Ok(value.cloned().unwrap_or_default());
            Ok(HttpResponse::Ok().json(&res))
        }
        e =>
            Ok(HttpResponse::Ok().json(&e))
    }
}
