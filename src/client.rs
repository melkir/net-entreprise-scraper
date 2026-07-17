use regex::Regex;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::LazyLock;
use worker::*;

const URL: &str = "https://www.net-entreprises.fr/declaration/outils-de-controle-dsn-val/";

static VERSION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)Version\s+(\d+(?:\.\d+)*)\s+du\s+(\d{1,2})(?:\s+(?:er|e))?\s+([\p{L}]+)\s+(\d{4})",
    )
    .unwrap()
});

static HTML_TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<[^>]+>").unwrap());

static SECTION_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<h2\b[^>]*>").unwrap());

static HREF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)href\s*=\s*["']([^"']+)["']"#).unwrap());

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct DsnToolInfo {
    version: String,
    date: String,
    urls: Vec<String>,
}

fn month_to_number(month: &str) -> Option<u32> {
    match month.trim().to_lowercase().as_str() {
        "janvier" => Some(1),
        "février" | "fevrier" => Some(2),
        "mars" => Some(3),
        "avril" => Some(4),
        "mai" => Some(5),
        "juin" => Some(6),
        "juillet" => Some(7),
        "août" | "aout" => Some(8),
        "septembre" => Some(9),
        "octobre" => Some(10),
        "novembre" => Some(11),
        "décembre" | "decembre" => Some(12),
        _ => None,
    }
}

fn is_valid_date(year: u32, month: u32, day: u32) -> bool {
    let is_leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days_in_month = match month {
        2 if is_leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => return false,
    };

    (1..=days_in_month).contains(&day)
}

fn normalize_download_url(raw_url: &str) -> Option<Url> {
    let decoded_url = raw_url
        .trim()
        .replace("&amp;", "&")
        .replace("&#038;", "&")
        .replace("&#38;", "&")
        .replace("&#x26;", "&")
        .replace("&#X26;", "&");
    let url = Url::parse(URL).ok()?.join(&decoded_url).ok()?;

    matches!(url.scheme(), "http" | "https").then_some(url)
}

fn is_download_url(url: &Url) -> bool {
    url.path()
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .is_some_and(|extension| matches!(extension.as_str(), "zip" | "exe" | "msi"))
}

fn extract_download_urls(section: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut seen = HashSet::new();

    for capture in HREF_RE.captures_iter(section) {
        let Some(raw_url) = capture.get(1).map(|m| m.as_str()) else {
            continue;
        };

        let Some(url) = normalize_download_url(raw_url) else {
            continue;
        };

        if !is_download_url(&url) {
            continue;
        }

        let url = url.to_string();
        if seen.insert(url.clone()) {
            urls.push(url);
        }
    }

    urls
}

fn parse_section(section: &str) -> Option<DsnToolInfo> {
    let text = HTML_TAG_RE.replace_all(section, " ");
    let version_caps = VERSION_RE.captures(&text)?;
    let build = version_caps.get(1)?.as_str();
    let day: u32 = version_caps.get(2)?.as_str().parse().ok()?;
    let month = month_to_number(version_caps.get(3)?.as_str())?;
    let year: u32 = version_caps.get(4)?.as_str().parse().ok()?;

    if !is_valid_date(year, month, day) {
        return None;
    }

    let urls = extract_download_urls(section);
    if urls.is_empty() {
        return None;
    }

    Some(DsnToolInfo {
        version: build.to_string(),
        date: format!("{year:04}-{month:02}-{day:02}"),
        urls,
    })
}

fn parse_page(body: &str) -> Vec<DsnToolInfo> {
    SECTION_RE.split(body).filter_map(parse_section).collect()
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
    let results = parse_page(&body);

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
            <a href='https://cdn.example.com/dsn-val.MSI?mirror=1&amp;source=api'>Msi</a>
            <a href='https://cdn.example.com/dsn-val.MSI?mirror=1&amp;source=api'>Duplicate</a>
        "#;

        let info = parse_section(section).unwrap();

        assert_eq!(
            info.urls,
            vec!["https://cdn.example.com/dsn-val.MSI?mirror=1&source=api".to_string()]
        );
        assert_eq!(info.date, "2025-02-14");
    }

    #[test]
    fn parse_page_handles_ordinal_date_markup() {
        let page = r#"
            <h2>Outil Dsn-Val 2026</h2>
            <p><strong>Version 2026.1.0.15 du 25 juin 2026</strong></p>
            <a href="https://cdn.example.com/dsn-val-2026.zip">Download</a>
            <h2 class="title">Outil Dsn-Val 2027</h2>
            <p><strong>Version 2027.1.0.2 du 1<sup>er</sup> juillet 2026</strong></p>
            <a href="https://cdn.example.com/dsn-val-2027.exe">Download</a>
        "#;

        let info = parse_page(page);

        assert_eq!(info.len(), 2);
        assert_eq!(info[1].version, "2027.1.0.2");
        assert_eq!(info[1].date, "2026-07-01");
    }

    #[test]
    fn parse_section_rejects_invalid_dates_and_non_http_downloads() {
        let invalid_date = r#"
            Version 2025.1 du 31 février 2025
            <a href="https://cdn.example.com/dsn-val.zip">Download</a>
        "#;
        let invalid_scheme = r#"
            Version 2025.1 du 28 février 2025
            <a href="javascript:dsn-val.zip">Download</a>
        "#;

        assert_eq!(parse_section(invalid_date), None);
        assert_eq!(parse_section(invalid_scheme), None);
    }
}
