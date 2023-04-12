use anyhow::Result;
use std::io;

use unicode_width::UnicodeWidthStr;

use crossterm::cursor;
use crossterm::event;
use crossterm::execute;
use crossterm::style;

#[non_exhaustive]
pub enum Role {
    Inactive,
    Active,
    Completed,
    Aborted,
}

#[non_exhaustive]
pub enum Status {
    Neutral,
    Uncertain,
    Valid,
    Invalid,
}

pub struct Config {
    pub prompt_text: String,
    pub prompt_hint: String,
}

pub trait Promptable
where
    Self: Sized,
{
    fn render_prompt(
        state: &mut Self,
        role: &Role,
        status: &Status,
        config: &Config,
        event: Option<event::KeyEvent>,
    ) -> Result<bool>;
    fn prompt(initial_state: Self, role: Role, status: Status, config: Config) -> Result<Self>;
}

impl Promptable for String {
    fn render_prompt(
        state: &mut Self,
        role: &Role,
        status: &Status,
        config: &Config,
        event: Option<event::KeyEvent>,
    ) -> Result<bool> {
        match event {
            Some(event) => match event.code {
                event::KeyCode::Char(c) => state.push(c),
                event::KeyCode::Backspace => drop(state.pop()),
                event::KeyCode::Enter => return Ok(true),
                _ => (),
            },
            None => (),
        };
        let state_len = u16::try_from(UnicodeWidthStr::width(state.as_str()))?;
        let prompt_len = u16::try_from(UnicodeWidthStr::width(config.prompt_text.as_str()))?;
        let hint_len = u16::try_from(UnicodeWidthStr::width(config.prompt_hint.as_str()))?;
        execute!(
            io::stdout(),
            // cursor::RestorePosition,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
            cursor::MoveToColumn(0)
        )?;
        execute!(
            io::stdout(),
            match status {
                Status::Neutral => style::SetForegroundColor(style::Color::Yellow),
                Status::Uncertain => style::SetForegroundColor(style::Color::Grey),
                Status::Valid => style::SetForegroundColor(style::Color::Green),
                Status::Invalid => style::SetForegroundColor(style::Color::Red),
            },
            style::Print(config.prompt_text.clone()),
            match role {
                Role::Inactive => style::SetForegroundColor(style::Color::Grey),
                Role::Active => style::SetForegroundColor(style::Color::White),
                Role::Completed => style::SetForegroundColor(style::Color::Green),
                Role::Aborted => style::SetForegroundColor(style::Color::Red),
            },
            match state.is_empty() {
                true => style::Print(config.prompt_hint.clone()),
                false => style::Print(state.to_string()),
            },
            match state.is_empty() {
                true => cursor::MoveToColumn(prompt_len),
                false => cursor::MoveToColumn(prompt_len + state_len),
            },
            style::ResetColor
        )?;
        Ok(false)
    }

    fn prompt(
        init_state: Self,
        init_role: Role,
        init_status: Status,
        init_config: Config,
    ) -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        let mut state = init_state;
        let role = init_role;
        let status = init_status;
        let config = init_config;
        Self::render_prompt(&mut state, &role, &status, &config, None)?;
        loop {
            match event::read()? {
                event::Event::Key(key) => {
                    if Self::render_prompt(&mut state, &role, &status, &config, Some(key))? {
                        break;
                    }
                }
                _ => (),
            }
        }
        crossterm::terminal::disable_raw_mode()?;
        execute!(io::stdout(), style::Print("\n"))?;
        Ok(state)
    }
}

// pub fn prompt_text() -> Result<String> {
//     execute!(io::stdout(), cursor::SavePosition)?;
//     execute!(io::stdout(), style::Print("Enter your name: ".to_string()))?;
//     let mut buffer = String::new();
//     let stdin = io::stdin();
//     stdin.read_line(&mut buffer)?;
//     execute!(io::stdout(), cursor::RestorePosition)?;
//     execute!(
//         io::stdout(),
//         style::Print("Thanks for giving me your name".to_string())
//     )?;
//     Ok(buffer)
// }
