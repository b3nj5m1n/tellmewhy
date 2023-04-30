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
#[derive(Debug)]
pub enum Status {
    Neutral,
    Uncertain,
    Valid,
    Invalid,
}

#[derive(Debug)]
pub struct Config {
    pub prompt_text: String,
    pub prompt_hint: String,
    pub max_width: Option<usize>,
}

#[derive(Debug)]
pub struct State<T> {
    pub input: Option<T>,
    pub cursor_position: usize,
}

fn get_width(config: &Config) -> Result<usize> {
    let terminal_width = usize::from(crossterm::terminal::size()?.0) - 1;
    Ok(match config.max_width {
        Some(x) => std::cmp::min(x, terminal_width),
        None => terminal_width,
    })
}

pub trait Promptable
where
    Self: Sized,
{
    fn render_prompt(
        role: &Role,
        status: &Status,
        config: &Config,
        event: Option<event::KeyEvent>,
        state: &mut State<Self>,
    ) -> Result<bool>;
    fn prompt(
        init_input: Option<Self>,
        init_role: Role,
        init_status: Status,
        init_config: Config,
    ) -> Result<Self>;
    fn get_length(state: &State<Self>) -> usize;
    fn move_cursor(state: &mut State<Self>, i: i8) {
        if i > 0 {
            state.cursor_position = (state.cursor_position
                + usize::try_from(i).expect("Unreachable"))
            .clamp(0, Self::get_length(state));
        } else {
            state.cursor_position = match state
                .cursor_position
                .checked_sub(usize::try_from(i.abs()).expect("Unreachable"))
            {
                Some(x) => x,
                None => 0,
            }
        }
    }
}

fn get_input_prefix(s: &String, i: usize) -> String {
    return s.chars().take(i.into()).collect();
}
fn get_input_suffix(s: &String, i: usize) -> String {
    return s.chars().skip(i.into()).collect();
}

fn insert_char_into_input(state: &mut State<String>, c: char) {
    match state.input.clone() {
        Some(input) => {
            let mut result = String::from("");
            result.push_str(&get_input_prefix(&input, state.cursor_position));
            result.push_str(&c.to_string());
            result.push_str(&get_input_suffix(&input, state.cursor_position));
            state.input = Some(result);
            // state.input.clear();
            // state.input.push_str(result.as_str())
            // s.clear();
            // s.push_str(result.as_str());
        }
        None => {
            state.input = Some(c.to_string());
        }
    }
}

fn remove_char_from_input(state: &mut State<String>) {
    if let Some(input) = state.input.clone() {
        let mut result = String::from("");
        result.push_str(&get_input_prefix(
            &input,
            match state.cursor_position.checked_sub(1) {
                Some(x) => x,
                None => 0,
            },
        ));
        result.push_str(&get_input_suffix(&input, state.cursor_position));
        state.input = match result.is_empty() {
            false => Some(result),
            true => None,
        };
        // s.clear();
        // s.push_str(result.as_str());
    }
}

fn truncate(s: String, width: usize) -> String {
    // println!("truncating {}", s);
    s.chars().take(width.into()).collect()
}

impl Promptable for String {
    fn render_prompt(
        role: &Role,
        status: &Status,
        config: &Config,
        event: Option<event::KeyEvent>,
        state: &mut State<Self>,
    ) -> Result<bool> {
        match event {
            Some(event) => match event.code {
                event::KeyCode::Char(c) => {
                    insert_char_into_input(state, c);
                    Self::move_cursor(state, 1);
                }
                event::KeyCode::Backspace => {
                    remove_char_from_input(state);
                    Self::move_cursor(state, -1);
                }
                event::KeyCode::Left => {
                    Self::move_cursor(state, -1);
                }
                event::KeyCode::Right => {
                    Self::move_cursor(state, 1);
                }
                event::KeyCode::Enter => return Ok(true),
                _ => (),
            },
            None => (),
        };
        // dbg!(&state);
        // let result_len = u16::try_from(UnicodeWidthStr::width(result.as_str()))?;
        let prompt_len = u16::try_from(UnicodeWidthStr::width(config.prompt_text.as_str()))?;
        let hint_len = u16::try_from(UnicodeWidthStr::width(config.prompt_hint.as_str()))?;
        let width = get_width(config)? - usize::from(prompt_len);
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
            match state.input.clone() {
                None => style::Print(truncate(config.prompt_hint.clone(), width)),
                Some(s) => style::Print(truncate(s, width)),
            },
            match state.input.clone() {
                None => cursor::MoveToColumn(prompt_len),
                Some(_) => cursor::MoveToColumn(
                    prompt_len
                        + u16::try_from(state.cursor_position)
                            .expect("Cursor position exceeds maximum 16 bit unsigned int value")
                ),
            },
            style::ResetColor
        )?;
        Ok(false)
    }

    fn prompt(
        init_input: Option<Self>,
        init_role: Role,
        init_status: Status,
        init_config: Config,
    ) -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        let role = init_role;
        let status = init_status;
        let config = init_config;
        let mut state = State {
            input: init_input,
            cursor_position: 0,
        };
        Self::render_prompt(&role, &status, &config, None, &mut state)?;
        loop {
            match event::read()? {
                event::Event::Key(key) => {
                    if Self::render_prompt(&role, &status, &config, Some(key), &mut state)? {
                        break;
                    }
                }
                _ => (),
            }
        }
        crossterm::terminal::disable_raw_mode()?;
        execute!(io::stdout(), style::Print("\n"))?;
        match state.input {
            Some(i) => Ok(i),
            None => Err(anyhow::anyhow!("Prompt exited without input")),
        }
    }

    fn get_length(state: &State<Self>) -> usize {
        match &state.input {
            Some(s) => unicode_width::UnicodeWidthStr::width(s.as_str()),
            None => 0,
        }
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
