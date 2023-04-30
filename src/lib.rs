use anyhow::Result;
use std::io;

use unicode_width::UnicodeWidthStr;

use crossterm::cursor;
use crossterm::event;
use crossterm::execute;
use crossterm::style;

#[non_exhaustive]
#[derive(Debug)]
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
    pub max_display_width: Option<usize>,
    pub max_length: Option<usize>,
}

#[derive(Debug)]
pub struct State<T> {
    pub input: Option<T>,
    pub cursor_position: usize,
    pub role: Role,
    pub status: Status,
}

fn get_width(config: &Config) -> Result<usize> {
    let terminal_width = usize::from(crossterm::terminal::size()?.0) - 1;
    Ok(match config.max_display_width {
        Some(x) => std::cmp::min(x, terminal_width),
        None => terminal_width,
    })
}

pub trait Promptable
where
    Self: Sized,
{
    fn render_prompt(
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
    fn validate(state: &State<Self>) -> Status;
    fn move_cursor(state: &mut State<Self>, i: i8) {
        if i > 0 {
            state.cursor_position = (state.cursor_position
                + usize::try_from(i).expect("Unreachable"))
            .clamp(0, Self::get_length(state));
        } else {
            state.cursor_position = state
                .cursor_position
                .saturating_sub(usize::try_from(i.abs()).expect("Unreachable"))
        }
    }
}

fn get_input_prefix(s: &str, i: usize) -> String {
    return s.chars().take(i).collect();
}
fn get_input_suffix(s: &str, i: usize) -> String {
    return s.chars().skip(i).collect();
}

fn insert_char_into_input(state: &mut State<String>, config: &Config, c: char) {
    if let Some(x) = config.max_length {
        if let Some(i) = state.input.clone() {
            if i.chars().count() == x {
                return;
            }
        }
    }
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

fn remove_char_from_input(state: &mut State<String>, _config: &Config) {
    if let Some(input) = state.input.clone() {
        let mut result = String::from("");
        result.push_str(&get_input_prefix(
            &input,
            state.cursor_position.saturating_sub(1),
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
    s.chars().take(width).collect()
}

impl Promptable for String {
    fn render_prompt(
        config: &Config,
        event: Option<event::KeyEvent>,
        state: &mut State<Self>,
    ) -> Result<bool> {
        if let Some(event) = event {
            match event.code {
                event::KeyCode::Char(c) => {
                    insert_char_into_input(state, config, c);
                    Self::move_cursor(state, 1);
                }
                event::KeyCode::Backspace => {
                    remove_char_from_input(state, config);
                    Self::move_cursor(state, -1);
                }
                event::KeyCode::Left => {
                    Self::move_cursor(state, -1);
                }
                event::KeyCode::Right => {
                    Self::move_cursor(state, 1);
                }
                event::KeyCode::Enter => {
                    if matches!(state.status, Status::Valid | Status::Neutral) {
                        return Ok(true);
                    }
                }
                _ => (),
            }
        };
        state.status = Self::validate(state);
        // dbg!(&state);
        // let result_len = u16::try_from(UnicodeWidthStr::width(result.as_str()))?;
        let prompt_len = u16::try_from(UnicodeWidthStr::width(config.prompt_text.as_str()))?;
        // let hint_len = u16::try_from(UnicodeWidthStr::width(config.prompt_hint.as_str()))?;
        let width = get_width(config)? - usize::from(prompt_len);
        execute!(
            io::stdout(),
            // cursor::RestorePosition,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
            cursor::MoveToColumn(0)
        )?;
        execute!(
            io::stdout(),
            match state.status {
                Status::Neutral => style::SetForegroundColor(style::Color::Yellow),
                Status::Uncertain => style::SetForegroundColor(style::Color::Grey),
                Status::Valid => style::SetForegroundColor(style::Color::Green),
                Status::Invalid => style::SetForegroundColor(style::Color::Red),
            },
            style::Print(config.prompt_text.clone()),
            match state.role {
                Role::Inactive => style::SetForegroundColor(style::Color::Grey),
                Role::Active => style::SetForegroundColor(style::Color::White),
                Role::Completed => style::SetForegroundColor(style::Color::Green),
                Role::Aborted => style::SetForegroundColor(style::Color::Red),
            },
            match state.input.clone() {
                None => style::Print(truncate(config.prompt_hint.clone(), width)),
                Some(s) => style::Print(truncate(s, width)),
            },
            cursor::MoveToColumn(
                prompt_len
                    + u16::try_from(state.cursor_position)
                        .expect("Cursor position exceeds maximum 16 bit unsigned int value")
            ),
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
        let config = init_config;
        let mut state = State {
            input: init_input,
            cursor_position: 0,
            role: init_role,
            status: init_status,
        };
        Self::render_prompt(&config, None, &mut state)?;
        loop {
            match event::read()? {
                event::Event::Key(key) => {
                    if Self::render_prompt(&config, Some(key), &mut state)? {
                        break;
                    }
                }
                event::Event::Resize(_, _) => {
                    if Self::render_prompt(&config, None, &mut state)? {
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

    fn validate(state: &State<Self>) -> Status {
        match state.input.clone() {
            Some(i) => match i.chars().any(|c| c.is_ascii_digit()) {
                true => Status::Invalid,
                false => Status::Valid,
            },
            None => Status::Uncertain,
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

#[cfg(test)]
mod tests {
    use crate::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn t_get_input_prefix() {
        assert_eq!(
            get_input_prefix(&String::from("test"), 4),
            String::from("test")
        );
        assert_eq!(
            get_input_prefix(&String::from("test"), 2),
            String::from("te")
        );
    }

    #[test]
    fn t_get_input_suffix() {
        assert_eq!(
            get_input_suffix(&String::from("test"), 2),
            String::from("st")
        );
        assert_eq!(get_input_suffix(&String::from("test"), 4), String::from(""));
    }

    #[test]
    fn t_insert_char_into_input_x_end() {
        let mut state: State<String> = State {
            input: Some("tes".into()),
            cursor_position: 4,
            role: Role::Active,
            status: Status::Neutral,
        };
        let config = Config {
            prompt_text: "".into(),
            prompt_hint: "".into(),
            max_display_width: None,
            max_length: None,
        };
        insert_char_into_input(&mut state, &config, 't');
        assert_eq!(state.input, Some("test".into()));
    }

    #[test]
    fn t_insert_char_into_input_x_beginning() {
        let mut state: State<String> = State {
            input: Some("est".into()),
            cursor_position: 0,
            role: Role::Active,
            status: Status::Neutral,
        };
        let config = Config {
            prompt_text: "".into(),
            prompt_hint: "".into(),
            max_display_width: None,
            max_length: None,
        };
        insert_char_into_input(&mut state, &config, 't');
        assert_eq!(state.input, Some("test".into()));
    }

    #[test]
    fn t_insert_char_into_input_x_middle() {
        let mut state: State<String> = State {
            input: Some("tst".into()),
            cursor_position: 1,
            role: Role::Active,
            status: Status::Neutral,
        };
        let config = Config {
            prompt_text: "".into(),
            prompt_hint: "".into(),
            max_display_width: None,
            max_length: None,
        };
        insert_char_into_input(&mut state, &config, 'e');
        assert_eq!(state.input, Some("test".into()));
    }

    #[test]
    fn t_insert_char_into_input_x_reaching_limit() {
        let mut state: State<String> = State {
            input: Some("test".into()),
            cursor_position: 0,
            role: Role::Active,
            status: Status::Neutral,
        };
        let config = Config {
            prompt_text: "".into(),
            prompt_hint: "".into(),
            max_display_width: None,
            max_length: Some(4),
        };
        insert_char_into_input(&mut state, &config, 't');
        assert_eq!(state.input, Some("test".into()));
    }

    #[test]
    fn t_remove_char_from_input_x_end() {
        let mut state: State<String> = State {
            input: Some("test".into()),
            cursor_position: 4,
            role: Role::Active,
            status: Status::Neutral,
        };
        let config = Config {
            prompt_text: "".into(),
            prompt_hint: "".into(),
            max_display_width: None,
            max_length: None,
        };
        remove_char_from_input(&mut state, &config);
        assert_eq!(state.input, Some("tes".into()));
    }

    #[test]
    fn t_remove_char_from_input_x_beginning() {
        let mut state: State<String> = State {
            input: Some("test".into()),
            cursor_position: 0,
            role: Role::Active,
            status: Status::Neutral,
        };
        let config = Config {
            prompt_text: "".into(),
            prompt_hint: "".into(),
            max_display_width: None,
            max_length: None,
        };
        remove_char_from_input(&mut state, &config);
        assert_eq!(state.input, Some("test".into()));
    }

    #[test]
    fn t_remove_char_from_input_x_middle() {
        let mut state: State<String> = State {
            input: Some("test".into()),
            cursor_position: 2,
            role: Role::Active,
            status: Status::Neutral,
        };
        let config = Config {
            prompt_text: "".into(),
            prompt_hint: "".into(),
            max_display_width: None,
            max_length: None,
        };
        remove_char_from_input(&mut state, &config);
        assert_eq!(state.input, Some("tst".into()));
    }

    #[test]
    fn t_truncate() {
        assert_eq!(truncate("test".into(), 2), "te".to_string());
    }
}
