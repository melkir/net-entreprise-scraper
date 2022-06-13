use axum::{
    response::{ErrorResponse, Json},
    routing::get,
    Router,
};
use serde_json::Value;
use sync_wrapper::SyncWrapper;

mod client;

async fn json() -> Result<Json<Value>, ErrorResponse> {
    let info = client::get_info().unwrap();
    Ok(Json(info))
}

#[shuttle_service::main]
async fn axum() -> shuttle_service::ShuttleAxum {
    let router = Router::new().route("/", get(json));
    let sync_wrapper = SyncWrapper::new(router);

    Ok(sync_wrapper)
}
