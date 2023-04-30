use anyhow::Result;
use tellmewhy::Promptable;

fn main() -> Result<()> {
    let name = String::prompt(
        // Some(String::new()),
        None,
        tellmewhy::Role::Active,
        tellmewhy::Status::Neutral,
        tellmewhy::Config {
            prompt_text: "Enter your name: ".into(),
            prompt_hint: "Firstname Lastname".into(),
            max_width: Some(30),
        },
    )?;
    println!("Hello, {name}");
    Ok(())
}
