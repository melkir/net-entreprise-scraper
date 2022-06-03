use scraper::{Html, Selector};

const URL: &str = "https://www.net-entreprises.fr/declaration/outils-de-controle-dsn-val/";

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resp = reqwest::get(URL).await?;
    let text = resp.text().await?;

    let document = Html::parse_document(&text);

    let link = get_link(&document);
    let version = get_version(&document);

    let info: Vec<&str> = version.split(' ').collect();
    let build = info[1];
    let day = info[3];
    let month = info[4];
    let year = info[5];

    let month_number = convert_month_to_number(month);

    println!("version: {}", build);

    let date = format!("{}-{}-{}", year, month_number, day);
    println!("date: {}", date);

    println!("url: {}", link);

    Ok(())
}
