use anyhow::Result;
use tellmewhy::Promptable;

fn main() -> Result<()> {
    let name = String::prompt(
        String::new(),
        tellmewhy::Role::Active,
        tellmewhy::Status::Neutral,
        tellmewhy::Config {
            prompt_text: "Enter your name: ".into(),
            prompt_hint: "Firstname Lastname".into(),
        },
    )?;
    println!("Hello, {name}");
    Ok(())
}
