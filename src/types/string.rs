use std::io;

use anyhow::Result;
use crossterm::{cursor, event, execute, style};
use unicode_width::UnicodeWidthStr;

use crate::*;

impl Promptable for String {
    fn render_prompt(
        config: &Config,
        event: Option<event::KeyEvent>,
        state: &mut State<Self>,
    ) -> Result<bool> {
        if let Some(event) = event {
            match event.code {
                event::KeyCode::Char(c) => {
                    util::insert_char_into_input(state, config, c);
                    Self::move_cursor(state, 1);
                }
                event::KeyCode::Backspace => {
                    util::remove_char_from_input(state, config);
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
        let full_width = get_width(config)?;
        let width = full_width.saturating_sub(usize::from(prompt_len));
        let input = util::truncate(config.prompt_hint.clone(), width, state.cursor_position);
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
                None => style::Print(input),
                Some(s) => style::Print(util::truncate(s, width, state.cursor_position)),
            },
            cursor::MoveToColumn(
                (prompt_len
                    + u16::try_from(state.cursor_position)
                        .expect("Cursor position exceeds maximum 16 bit unsigned int value"))
                .min(
                    u16::try_from(full_width)
                        .expect("Cursor position exceeds maximum 16 bit unsigned int value")
                )
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
    ) -> Result<Message<Self>> {
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
            Some(i) => Ok(Message::Result((i))),
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
