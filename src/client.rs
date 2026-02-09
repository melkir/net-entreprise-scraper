use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;
use worker::*;

const URL: &str = "https://www.net-entreprises.fr/declaration/outils-de-controle-dsn-val/";

static VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Version\s+([\d.]+)\s+du\s+(\d{1,2})\s+(\w+)\s+(\d{4})").unwrap());

static DOWNLOAD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"href="([^"]*\.(?:zip|exe|msi))""#).unwrap());

#[derive(Debug, Serialize)]
pub struct DsnToolInfo {
    version: String,
    date: String,
    urls: Vec<String>,
}

fn month_to_number(month: &str) -> Option<&'static str> {
    match month {
        "janvier" => Some("01"),
        "février" | "fevrier" => Some("02"),
        "mars" => Some("03"),
        "avril" => Some("04"),
        "mai" => Some("05"),
        "juin" => Some("06"),
        "juillet" => Some("07"),
        "août" | "aout" => Some("08"),
        "septembre" => Some("09"),
        "octobre" => Some("10"),
        "novembre" => Some("11"),
        "décembre" | "decembre" => Some("12"),
        _ => None,
    }
}

fn parse_section(section: &str) -> Option<DsnToolInfo> {
    let version_caps = VERSION_RE.captures(section)?;
    let build = version_caps.get(1)?.as_str();
    let day: u32 = version_caps.get(2)?.as_str().parse().ok()?;
    let month = month_to_number(version_caps.get(3)?.as_str())?;
    let year = version_caps.get(4)?.as_str();

    let urls: Vec<String> = DOWNLOAD_RE
        .captures_iter(section)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if urls.is_empty() {
        return None;
    }

    Some(DsnToolInfo {
        version: build.to_string(),
        date: format!("{}-{}-{:02}", year, month, day),
        urls,
    })
}

pub async fn get_info() -> worker::Result<Vec<DsnToolInfo>> {
    let mut init = RequestInit::new();
    init.method = Method::Get;

    let request = Request::new_with_init(URL, &init)?;
    let mut response = Fetch::Request(request).send().await?;
    let body = response.text().await?;

    let sections: Vec<&str> = body.split("<h2").collect();
    let results: Vec<DsnToolInfo> = sections.into_iter().filter_map(parse_section).collect();

    if results.is_empty() {
        return Err(worker::Error::RustError(
            "No version information found on the page".to_string(),
        ));
    }

    Ok(results)
}
