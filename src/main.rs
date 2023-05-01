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
            max_display_width: Some(25),
            max_length: None,
        },
    )?;
    println!("Hello, {name}");
    Ok(())
}
