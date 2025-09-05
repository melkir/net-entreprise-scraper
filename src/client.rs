use regex::Regex;
use serde_json::{json, Value};
use worker::*;

const URL: &str = "https://www.net-entreprises.fr/declaration/outils-de-controle-dsn-val/";

fn get_link(html: &str) -> Option<String> {
    // Find the last href attribute within strong tags
    let re =
        Regex::new(r#"<strong[^>]*>.*?<a[^>]*href="([^"]*)"[^>]*>.*?</a>.*?</strong>"#).ok()?;
    re.captures_iter(html)
        .last()?
        .get(1)
        .map(|m| m.as_str().trim().to_string())
}

fn get_version(html: &str) -> Option<String> {
    // Find text content of strong tags that contain "Version 20"
    let re = Regex::new(r#"<strong[^>]*>([^<]*Version 20[^<]*)</strong>"#).ok()?;
    let captures: Vec<_> = re.captures_iter(html).collect();
    captures
        .last()?
        .get(1)
        .map(|m| m.as_str().trim().to_string())
}

fn convert_month_to_number(month: &str) -> &str {
    match month {
        "janvier" => "01",
        "février" => "02",
        "mars" => "03",
        "avril" => "04",
        "mai" => "05",
        "juin" => "06",
        "juillet" => "07",
        "août" => "08",
        "septembre" => "09",
        "octobre" => "10",
        "novembre" => "11",
        "décembre" => "12",
        _ => "01", // fallback instead of panic
    }
}

pub async fn get_info() -> worker::Result<Value> {
    let mut init = RequestInit::new();
    init.method = Method::Get;

    let request = Request::new_with_init(URL, &init)?;
    let mut response = Fetch::Request(request).send().await?;
    let body = response.text().await?;

    let link = get_link(&body).unwrap_or_default();
    let version_text = get_version(&body).unwrap_or_default();

    let info: Vec<&str> = version_text.split(' ').collect();
    if info.len() < 6 {
        return Err(worker::Error::RustError(
            "Invalid version format".to_string(),
        ));
    }

    let build = info[1];
    let day = info[3];
    let month = info[4];
    let year = info[5];
    let month_number = convert_month_to_number(month);

    let json = json!({
        "version": build,
        "date": format!("{}-{}-{}", year, month_number, day),
        "url": link,
    });

    Ok(json)
}
