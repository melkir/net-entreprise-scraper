mod client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let info = client::get_info()?;
    println!("{}", info);
    Ok(())
}
