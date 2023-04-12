use anyhow::Result;
use std::io;

pub fn prompt_text() -> Result<String> {
    let mut buffer = String::new();
    let stdin = io::stdin();
    stdin.read_line(&mut buffer)?;
    Ok(buffer)
}
