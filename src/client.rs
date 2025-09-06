use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use worker::*;

const URL: &str = "https://www.net-entreprises.fr/declaration/outils-de-controle-dsn-val/";

#[derive(Debug, Serialize, Deserialize)]
struct DsnToolInfo {
    version: String,
    date: String,
    url: String,
}

fn get_all_links(html: &str) -> Vec<String> {
    // Find all href attributes within strong tags that contain download links
    let re = Regex::new(
        r#"<strong[^>]*>.*?<a[^>]*href="([^"]*\.(?:zip|exe|msi))"[^>]*>.*?</a>.*?</strong>"#,
    )
    .unwrap();
    re.captures_iter(html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
        .filter(|url| !url.is_empty())
        .collect()
}

fn get_all_versions(html: &str) -> Vec<String> {
    // Find all text content of strong tags that contain "Version 20" followed by date
    let re = Regex::new(r#"<strong[^>]*>([^<]*Version 20\d{2}[^<]*\d{4}[^<]*)</strong>"#).unwrap();
    re.captures_iter(html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
        .filter(|text| !text.is_empty())
        .collect()
}

fn convert_month_to_number(month: &str) -> Option<&str> {
    match month {
        "janvier" => Some("01"),
        "février" => Some("02"),
        "mars" => Some("03"),
        "avril" => Some("04"),
        "mai" => Some("05"),
        "juin" => Some("06"),
        "juillet" => Some("07"),
        "août" => Some("08"),
        "septembre" => Some("09"),
        "octobre" => Some("10"),
        "novembre" => Some("11"),
        "décembre" => Some("12"),
        _ => None,
    }
}

fn parse_version_info(version_text: &str, url: &str) -> worker::Result<DsnToolInfo> {
    let info: Vec<&str> = version_text.split_whitespace().collect();
    if info.len() < 6 {
        return Err(worker::Error::RustError(format!(
            "Invalid version format - expected at least 6 parts, got {}: '{}'",
            info.len(),
            version_text
        )));
    }

    let build = info[1];
    let day = info[3];
    let month = info[4];
    let year = info[5];

    // Validate day is numeric and reasonable
    if day.parse::<u32>().map_or(true, |d| !(1..=31).contains(&d)) {
        return Err(worker::Error::RustError(format!(
            "Invalid day value: '{}'",
            day
        )));
    }

    // Validate year is 4 digits
    if year.len() != 4 || year.parse::<u32>().is_err() {
        return Err(worker::Error::RustError(format!(
            "Invalid year value: '{}'",
            year
        )));
    }

    let month_number = convert_month_to_number(month)
        .ok_or_else(|| worker::Error::RustError(format!("Unrecognized month: '{}'", month)))?;

    Ok(DsnToolInfo {
        version: build.to_string(),
        date: format!(
            "{}-{:02}-{:02}",
            year,
            month_number.parse::<u32>().unwrap_or(1),
            day.parse::<u32>().unwrap_or(1)
        ),
        url: url.to_string(),
    })
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

    let results: worker::Result<Vec<DsnToolInfo>> = links
        .iter()
        .zip(versions.iter())
        .map(|(link, version_text)| parse_version_info(version_text, link))
        .collect();

    Ok(json!(results?))
}
