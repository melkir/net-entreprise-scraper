use std::sync::Arc;

use scraper::{Html, Selector};
use serde_json::{json, Value};

const URL: &str = "https://www.net-entreprises.fr/declaration/outils-de-controle-dsn-val/";

// Implementation of `ServerCertVerifier` that verifies everything as trustworthy.
struct SkipServerVerification;

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}

fn get_link(document: &Html) -> &str {
    let selector = Selector::parse(r#"strong > a"#).unwrap();
    let url = document
        .select(&selector)
        .last()
        .expect("selector not found");

    url.value().attr("href").expect("href not found").trim()
}

fn get_version(document: &Html) -> &str {
    let selector = Selector::parse(r#"td > p > strong"#).unwrap();
    let version = document
        .select(&selector)
        .next()
        .expect("selector not found");

    version.text().next().expect("text not found").trim()
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
        _ => panic!("unknown month"),
    }
}

pub fn get_info() -> Result<Value, Box<dyn std::error::Error>> {
    let tls_config = rustls::ClientConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_safe_default_protocol_versions()?
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();

    let agent = ureq::builder().tls_config(Arc::new(tls_config)).build();
    let body: String = agent.get(URL).call()?.into_string()?;
    let document = Html::parse_document(&body);
    let link = get_link(&document);
    let version = get_version(&document);
    let info: Vec<&str> = version.split(' ').collect();
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
