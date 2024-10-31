use axum::{
    response::{ErrorResponse, Json},
    routing::get,
    Router,
};
use serde_json::Value;

mod client;

async fn json() -> Result<Json<Value>, ErrorResponse> {
    let info = client::get_info().unwrap();
    Ok(Json(info))
}

#[shuttle_runtime::main]
async fn axum() -> shuttle_axum::ShuttleAxum {
    let router = Router::new().route("/", get(json));

    Ok(router.into())
}
