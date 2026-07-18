use worker::*;

mod client;

const CACHE_CONTROL: &str =
    "public, max-age=300, stale-while-revalidate=86400, stale-if-error=86400";

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    Router::new()
        .get_async("/", handle_root)
        .run(req, env)
        .await
}

async fn handle_root(_req: Request, _route: RouteContext<()>) -> Result<Response> {
    match client::get_info().await {
        Ok(info) => {
            let mut response = Response::from_json(&info)?;
            response.headers_mut().set("Cache-Control", CACHE_CONTROL)?;

            Ok(response)
        }
        Err(error) => {
            console_error!("DSN tool retrieval failed: {error}");
            Response::error("Failed to retrieve DSN tool information", 502)
        }
    }
}
