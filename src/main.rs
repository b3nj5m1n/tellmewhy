use anyhow::Result;
use tellmewhy::prompt_text;

fn main() -> Result<()> {
    let name = prompt_text()?;
    println!("Hello, {name}");
    Ok(())
}
