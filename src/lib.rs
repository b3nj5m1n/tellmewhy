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

pub struct State {
    pub cursor_position: u16,
}

pub trait Promptable
where
    Self: Sized,
{
    fn render_prompt(
        result: &mut Self,
        role: &Role,
        status: &Status,
        config: &Config,
        event: Option<event::KeyEvent>,
        state: &mut State,
    ) -> Result<bool>;
    fn prompt(initial_result: Self, role: Role, status: Status, config: Config) -> Result<Self>;
}

fn get_input_prefix(s: String, i: u16) -> String {
    return s.chars().take(i.into()).collect();
}
fn get_input_suffix(s: String, i: u16) -> String {
    return s.chars().skip(i.into()).collect();
}

fn insert_char_into_input(s: &mut String, i: u16, c: char) {
    let mut result = String::from("");
    result.push_str(&get_input_prefix(s.to_string(), i));
    result.push_str(&c.to_string());
    result.push_str(&get_input_suffix(s.to_string(), i));
    s.clear();
    s.push_str(result.as_str());
}

fn remove_char_from_input(s: &mut String, i: u16) {
    let mut result = String::from("");
    result.push_str(&get_input_prefix(
        s.to_string(),
        match i.checked_sub(1) {
            Some(x) => x,
            None => 0,
        },
    ));
    result.push_str(&get_input_suffix(s.to_string(), i));
    s.clear();
    s.push_str(result.as_str());
}

fn truncate(s: String, width: u16) -> String {
    s.chars().take(width.into()).collect()
}

fn move_cursor(state: &mut State, i: i8, width: u16) {
    if i > 0 {
        state.cursor_position =
            (state.cursor_position + u16::try_from(i).expect("Unreachable")).clamp(0, width);
    } else {
        state.cursor_position = match state
            .cursor_position
            .checked_sub(u16::try_from(i.abs()).expect("Unreachable"))
        {
            Some(x) => x,
            None => 0,
        }
    }
}

impl Promptable for String {
    fn render_prompt(
        result: &mut Self,
        role: &Role,
        status: &Status,
        config: &Config,
        event: Option<event::KeyEvent>,
        state: &mut State,
    ) -> Result<bool> {
        let width = 30;
        match event {
            Some(event) => match event.code {
                event::KeyCode::Char(c) => {
                    insert_char_into_input(result, state.cursor_position, c);
                    move_cursor(state, 1, width);
                }
                event::KeyCode::Backspace => {
                    remove_char_from_input(result, state.cursor_position);
                    move_cursor(state, -1, width);
                }
                event::KeyCode::Left => {
                    move_cursor(state, -1, width);
                }
                event::KeyCode::Right => {
                    move_cursor(state, 1, width);
                }
                event::KeyCode::Enter => return Ok(true),
                _ => (),
            },
            None => (),
        };
        let result_len = u16::try_from(UnicodeWidthStr::width(result.as_str()))?;
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
            match result.is_empty() {
                true => style::Print(truncate(config.prompt_hint.clone(), width)),
                false => style::Print(truncate(result.to_string(), width)),
            },
            match result.is_empty() {
                true => cursor::MoveToColumn(prompt_len),
                false => cursor::MoveToColumn(prompt_len + state.cursor_position),
            },
            style::ResetColor
        )?;
        Ok(false)
    }

    fn prompt(
        init_result: Self,
        init_role: Role,
        init_status: Status,
        init_config: Config,
    ) -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        let mut result = init_result;
        let role = init_role;
        let status = init_status;
        let config = init_config;
        let mut state = State { cursor_position: 0 };
        Self::render_prompt(&mut result, &role, &status, &config, None, &mut state)?;
        loop {
            match event::read()? {
                event::Event::Key(key) => {
                    if Self::render_prompt(
                        &mut result,
                        &role,
                        &status,
                        &config,
                        Some(key),
                        &mut state,
                    )? {
                        break;
                    }
                }
                _ => (),
            }
        }
        crossterm::terminal::disable_raw_mode()?;
        execute!(io::stdout(), style::Print("\n"))?;
        Ok(result)
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
