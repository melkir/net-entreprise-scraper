use axum::{
    response::{ErrorResponse, Json},
    routing::get,
    Router,
    serve,
};
use serde_json::Value;
use std::net::SocketAddr;

mod client;

async fn json() -> Result<Json<Value>, ErrorResponse> {
    let info = client::get_info().unwrap();
    Ok(Json(info))
}

#[tokio::main]
async fn main() {
    let router = Router::new().route("/", get(json));

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse()
        .unwrap();

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    serve(listener, router).await.unwrap();
}
