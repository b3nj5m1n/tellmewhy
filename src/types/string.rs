use std::io;

use anyhow::Result;
use crossterm::{cursor, event, execute, style};
use unicode_width::UnicodeWidthStr;

use crate::*;

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

fn truncate(s: String, width: usize, cursor_position: usize) -> (String, usize) {
    // TODO this is scuffed
    // FIXME doesn't respect width (will be 1 more than width when cutoff during scroll to the right)
    let len = s.chars().count();
    let ellipsis = "…";
    let ellipsis_len = ellipsis.chars().count();
    let min_width = 2 * ellipsis_len + 1;

    if len >= width {
        let start = cursor_position.saturating_sub(width);
        let end = start + width;
        let display_ellipsis = width > min_width;

        let mut result = s.chars().skip(start).take(end - start).collect::<String>();

        if start > 0 && display_ellipsis {
            result.remove(0);
            result.insert_str(0, ellipsis);
        }

        if end < len && display_ellipsis {
            // result.pop();
            result.push_str(ellipsis);
        }

        (result, width)
    } else {
        (s.chars().take(width).collect(), width)
    }
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
        let full_width = get_width(config)?;
        let width = full_width.saturating_sub(usize::from(prompt_len));
        let (input, cursor_move) =
            truncate(config.prompt_hint.clone(), width, state.cursor_position);
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
                Some(s) => style::Print(truncate(s, width, state.cursor_position).0),
            },
            cursor::MoveToColumn(
                (prompt_len
                    + u16::try_from(state.cursor_position.min(cursor_move))
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

#[cfg(test)]
mod tests {
    use crate::types::string::*;
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
    fn t_truncate_x_normal() {
        assert_eq!(truncate("test".into(), 2, 0), ("te".to_string(), 2));
    }

    #[test]
    fn t_truncate_x_scroll_middle() {
        assert_eq!(truncate("test123".into(), 2, 5), ("t1".to_string(), 2));
    }

    #[test]
    fn t_truncate_x_scroll_end() {
        assert_eq!(truncate("test123".into(), 2, 7), ("23".to_string(), 2));
    }

    #[test]
    fn t_truncate_x_normal_ellipsis() {
        // Doesn't properly respect width but best I can do right now
        assert_eq!(truncate("test123".into(), 4, 0), ("test…".to_string(), 4));
    }

    #[test]
    fn t_truncate_x_scroll_middle_ellipsis() {
        assert_eq!(truncate("test123".into(), 4, 5), ("…st1…".to_string(), 4));
    }

    #[test]
    fn t_truncate_x_scroll_end_ellipsis() {
        assert_eq!(truncate("test123".into(), 4, 7), ("…123".to_string(), 4));
    }
}
