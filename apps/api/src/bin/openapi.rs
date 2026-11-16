//! Prints the OpenAPI document. CI compares it with `openapi/openapi.json`:
//! `cargo run -q -p cyex-tv-api --bin openapi > openapi/openapi.json`

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json = cyex_tv_api::openapi().to_pretty_json()?;
    println!("{json}");
    Ok(())
}
