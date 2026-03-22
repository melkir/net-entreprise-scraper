use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;
use worker::*;

const URL: &str = "https://www.net-entreprises.fr/declaration/outils-de-controle-dsn-val/";

static VERSION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Version\s+([\d.]+)\s+du\s+(\d{1,2})\s+([\p{L}]+)\s+(\d{4})").unwrap()
});

static DOWNLOAD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"href=["']([^"']*\.(?:zip|exe|msi))["']"#).unwrap());

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct DsnToolInfo {
    version: String,
    date: String,
    urls: Vec<String>,
}

fn month_to_number(month: &str) -> Option<&'static str> {
    match month.trim().to_lowercase().as_str() {
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

fn normalize_download_url(url: &str) -> Option<String> {
    Url::parse(URL)
        .ok()?
        .join(url)
        .ok()
        .map(|url| url.to_string())
}

fn extract_download_urls(section: &str) -> Vec<String> {
    let mut urls = Vec::new();

    for capture in DOWNLOAD_RE.captures_iter(section) {
        let Some(raw_url) = capture.get(1).map(|m| m.as_str()) else {
            continue;
        };

        let Some(url) = normalize_download_url(raw_url) else {
            continue;
        };

        if !urls.contains(&url) {
            urls.push(url);
        }
    }

    urls
}

fn parse_section(section: &str) -> Option<DsnToolInfo> {
    let version_caps = VERSION_RE.captures(section)?;
    let build = version_caps.get(1)?.as_str();
    let day: u32 = version_caps.get(2)?.as_str().parse().ok()?;
    let month = month_to_number(version_caps.get(3)?.as_str())?;
    let year = version_caps.get(4)?.as_str();

    let urls = extract_download_urls(section);
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

    if !(200..=299).contains(&response.status_code()) {
        return Err(Error::RustError(format!(
            "Upstream request failed with status {}",
            response.status_code()
        )));
    }

    let body = response.text().await?;
    let results: Vec<DsnToolInfo> = body.split("<h2").filter_map(parse_section).collect();

    if results.is_empty() {
        return Err(worker::Error::RustError(
            "No version information found on the page".to_string(),
        ));
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_section_extracts_date_and_absolute_urls() {
        let section = r#"
            <h2>Version 2025.1 du 3 février 2025</h2>
            <a href="/files/dsn-val.zip">Zip</a>
            <a href="installer/setup.exe">Exe</a>
        "#;

        let info = parse_section(section).unwrap();

        assert_eq!(
            info,
            DsnToolInfo {
                version: "2025.1".to_string(),
                date: "2025-02-03".to_string(),
                urls: vec![
                    "https://www.net-entreprises.fr/files/dsn-val.zip".to_string(),
                    "https://www.net-entreprises.fr/declaration/outils-de-controle-dsn-val/installer/setup.exe".to_string(),
                ],
            }
        );
    }

    #[test]
    fn parse_section_supports_single_quoted_links_and_deduplicates_urls() {
        let section = r#"
            <h2>Version 2025.2 du 14 Fevrier 2025</h2>
            <a href='https://cdn.example.com/dsn-val.msi'>Msi</a>
            <a href='https://cdn.example.com/dsn-val.msi'>Duplicate</a>
        "#;

        let info = parse_section(section).unwrap();

        assert_eq!(
            info.urls,
            vec!["https://cdn.example.com/dsn-val.msi".to_string()]
        );
        assert_eq!(info.date, "2025-02-14");
    }
}
