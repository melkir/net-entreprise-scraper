use regex::Regex;
use serde_json::{json, Value};
use worker::*;

const URL: &str = "https://www.net-entreprises.fr/declaration/outils-de-controle-dsn-val/";

fn get_all_links(html: &str) -> Vec<String> {
    // Find all href attributes within strong tags
    let re =
        Regex::new(r#"<strong[^>]*>.*?<a[^>]*href="([^"]*)"[^>]*>.*?</a>.*?</strong>"#).unwrap();
    re.captures_iter(html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
        .collect()
}

fn get_all_versions(html: &str) -> Vec<String> {
    // Find all text content of strong tags that contain "Version 20"
    let re = Regex::new(r#"<strong[^>]*>([^<]*Version 20[^<]*)</strong>"#).unwrap();
    re.captures_iter(html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
        .collect()
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

    let links = get_all_links(&body);
    let versions = get_all_versions(&body);

    if links.len() != versions.len() {
        return Err(worker::Error::RustError(
            "Mismatch between number of links and versions found".to_string(),
        ));
    }

    let mut results = Vec::new();

    for (link, version_text) in links.iter().zip(versions.iter()) {
        let info: Vec<&str> = version_text.split(' ').collect();
        if info.len() < 6 {
            return Err(worker::Error::RustError(format!(
                "Invalid version format: {}",
                version_text
            )));
        }

        let build = info[1];
        let day = info[3];
        let month = info[4];
        let year = info[5];
        let month_number = convert_month_to_number(month);

        let entry = json!({
            "version": build,
            "date": format!("{}-{}-{}", year, month_number, day),
            "url": link,
        });

        results.push(entry);
    }

    Ok(json!(results))
}
