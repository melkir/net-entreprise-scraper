use worker::*;

mod client;

const CACHE_TTL: u64 = 300; // 5 minutes

#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    Router::new()
        .get_async("/", |req, ctx| async move {
            let cache = Cache::default();
            let cache_request = Request::new_with_init(
                req.url()?.as_str(),
                RequestInit::new().with_method(Method::Get),
            )?;

            // Try to get from cache first
            if let Ok(Some(cached_response)) = cache.get(&cache_request, false).await {
                return Ok(cached_response);
            }

            // Cache miss - fetch fresh data
            match client::get_info().await {
                Ok(info) => {
                    let mut response = Response::from_json(&info)?;

                    // Set cache headers
                    let headers = response.headers_mut();
                    headers.set("Cache-Control", &format!("public, max-age={}", CACHE_TTL))?;

                    // Store in cache
                    let _ = cache.put(&cache_request, response.cloned()?).await;

                    Ok(response)
                }
                Err(e) => Response::error(format!("Error: {}", e), 500),
            }
        })
        .run(req, env)
        .await
}
