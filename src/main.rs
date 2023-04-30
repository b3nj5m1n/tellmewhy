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
            max_display_width: Some(50),
            max_length: Some(5),
        },
    )?;
    println!("Hello, {name}");
    Ok(())
}
