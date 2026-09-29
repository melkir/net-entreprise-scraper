use regex::Regex;
use serde::Serialize;
use std::borrow::Cow;
use std::sync::LazyLock;
use worker::{Error, Fetch, Result, Url};

static PAGE_URL: LazyLock<Url> = LazyLock::new(|| {
    Url::parse("https://www.net-entreprises.fr/declaration/outils-de-controle-dsn-val/").unwrap()
});

static VERSION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)Version\s+(\d+(?:\.\d+)*)\s+du\s+(\d{1,2})(?:\s*er?)?\s+(\p{L}+)\s+(\d{4})")
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

const DOWNLOAD_EXTENSIONS: [&str; 3] = ["zip", "exe", "msi"];

// `&amp;` is decoded last so that escaped entities such as `&amp;nbsp;` are not decoded twice.
const HTML_ENTITIES: [(&str, &str); 7] = [
    ("&nbsp;", " "),
    ("&#160;", " "),
    ("&#038;", "&"),
    ("&#38;", "&"),
    ("&#x26;", "&"),
    ("&#X26;", "&"),
    ("&amp;", "&"),
];

fn decode_entities(text: &str) -> Cow<'_, str> {
    if !text.contains('&') {
        return Cow::Borrowed(text);
    }

    let mut decoded = text.to_string();
    for (entity, replacement) in HTML_ENTITIES {
        decoded = decoded.replace(entity, replacement);
    }

    Cow::Owned(decoded)
}

fn month_to_number(month: &str) -> Option<u32> {
    match month.to_lowercase().as_str() {
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
    let url = PAGE_URL.join(&decode_entities(raw_url.trim())).ok()?;

    matches!(url.scheme(), "http" | "https").then_some(url)
}

fn is_download_url(url: &Url) -> bool {
    url.path().rsplit_once('.').is_some_and(|(_, extension)| {
        DOWNLOAD_EXTENSIONS
            .iter()
            .any(|download_extension| extension.eq_ignore_ascii_case(download_extension))
    })
}

fn extract_download_urls(section: &str) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();

    let candidates = HREF_RE
        .captures_iter(section)
        .filter_map(|capture| normalize_download_url(&capture[1]))
        .filter(is_download_url)
        .map(String::from);

    // Sections only hold a handful of links, so a linear scan beats hashing.
    for url in candidates {
        if !urls.contains(&url) {
            urls.push(url);
        }
    }

    urls
}

fn parse_section(section: &str) -> Option<DsnToolInfo> {
    let without_tags = HTML_TAG_RE.replace_all(section, " ");
    let text = decode_entities(&without_tags);
    let caps = VERSION_RE.captures(&text)?;
    let day: u32 = caps[2].parse().ok()?;
    let month = month_to_number(&caps[3])?;
    let year: u32 = caps[4].parse().ok()?;

    if !is_valid_date(year, month, day) {
        return None;
    }

    let urls = extract_download_urls(section);
    if urls.is_empty() {
        return None;
    }

    Some(DsnToolInfo {
        version: caps[1].to_string(),
        date: format!("{year:04}-{month:02}-{day:02}"),
        urls,
    })
}

fn parse_page(body: &str) -> Vec<DsnToolInfo> {
    SECTION_RE.split(body).filter_map(parse_section).collect()
}

pub async fn get_info() -> Result<Vec<DsnToolInfo>> {
    let mut response = Fetch::Url(PAGE_URL.clone()).send().await?;

    let status = response.status_code();
    if !(200..=299).contains(&status) {
        return Err(Error::RustError(format!(
            "Upstream request failed with status {status}"
        )));
    }

    let results = parse_page(&response.text().await?);

    if results.is_empty() {
        return Err(Error::RustError(
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
    fn parse_page_keeps_downloads_within_their_version_section() {
        let page = r#"
            <h2>Incomplete version</h2>
            <p>Version 2026.1 du 4 mars 2026</p>
            <h2>Unrelated download</h2>
            <a href="https://cdn.example.com/unrelated.zip">Download</a>
            <h2>Complete version</h2>
            <p>Version 2026.2 du 5 mars 2026</p>
            <a href="https://cdn.example.com/dsn-val-2026.2.zip">Download</a>
        "#;

        let info = parse_page(page);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].version, "2026.2");
        assert_eq!(
            info[0].urls,
            vec!["https://cdn.example.com/dsn-val-2026.2.zip"]
        );
    }

    #[test]
    fn parse_section_handles_unspaced_ordinal_and_nbsp_dates() {
        let unspaced_ordinal = r#"
            <h2>Version 2027.1.0.2 du 1er juillet 2026</h2>
            <a href="https://cdn.example.com/dsn-val.zip">Download</a>
        "#;
        let nbsp = r#"
            <h2>Version 2026.2 du 5&nbsp;mars&nbsp;2026</h2>
            <a href="https://cdn.example.com/dsn-val.zip">Download</a>
        "#;

        assert_eq!(parse_section(unspaced_ordinal).unwrap().date, "2026-07-01");
        assert_eq!(parse_section(nbsp).unwrap().date, "2026-03-05");
    }

    #[test]
    fn parse_section_handles_leap_year_boundaries() {
        let leap_day = r#"
            Version 2024.2 du 29 février 2024
            <a href="https://cdn.example.com/dsn-val.zip">Download</a>
        "#;
        let non_leap_day = r#"
            Version 2100.2 du 29 février 2100
            <a href="https://cdn.example.com/dsn-val.zip">Download</a>
        "#;

        assert_eq!(parse_section(leap_day).unwrap().date, "2024-02-29");
        assert_eq!(parse_section(non_leap_day), None);
    }

    #[test]
    fn dsn_tool_info_serializes_to_the_api_contract() {
        let info = DsnToolInfo {
            version: "2026.2".to_string(),
            date: "2026-03-05".to_string(),
            urls: vec!["https://cdn.example.com/dsn-val.zip".to_string()],
        };

        assert_eq!(
            serde_json::to_value(info).unwrap(),
            serde_json::json!({
                "version": "2026.2",
                "date": "2026-03-05",
                "urls": ["https://cdn.example.com/dsn-val.zip"]
            })
        );
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
