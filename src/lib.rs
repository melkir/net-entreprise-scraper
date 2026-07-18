use worker::*;

mod client;

const CACHE_TTL: u64 = 300; // 5 minutes

#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    Router::with_data(ctx)
        .get_async("/", handle_root)
        .run(req, env)
        .await
}

async fn handle_root(req: Request, route: RouteContext<worker::Context>) -> Result<Response> {
    let cache = Cache::default();
    let cache_request = create_cache_request(&req)?;

    match cache.get(&cache_request, false).await {
        Ok(Some(cached_response)) => return Ok(cached_response),
        Ok(None) => {}
        Err(error) => console_error!("Cache lookup failed: {error}"),
    }

    match client::get_info().await {
        Ok(info) => {
            let mut response = Response::from_json(&info)?;
            response
                .headers_mut()
                .set("Cache-Control", &format!("public, max-age={CACHE_TTL}"))?;

            let cache_response = response.cloned()?;
            route.data.wait_until(async move {
                if let Err(error) = cache.put(&cache_request, cache_response).await {
                    console_error!("Cache update failed: {error}");
                }
            });

            Ok(response)
        }
        Err(error) => {
            console_error!("DSN tool retrieval failed: {error}");
            Response::error("Failed to retrieve DSN tool information", 502)
        }
    }
}

fn create_cache_request(req: &Request) -> Result<Request> {
    let cache_url = canonical_cache_url(req.url()?);
    Request::new_with_init(&cache_url, RequestInit::new().with_method(Method::Get))
}

fn canonical_cache_url(mut url: Url) -> String {
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_url_ignores_unused_query_and_fragment() {
        let url =
            Url::parse("https://api.example.com/releases/latest?cache-bust=1#result").unwrap();

        assert_eq!(
            canonical_cache_url(url),
            "https://api.example.com/releases/latest"
        );
    }
}
