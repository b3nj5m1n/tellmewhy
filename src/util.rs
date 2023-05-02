use crate::*;

// Utility functions for string-based prompts

/// Get count number of characters from the beginning of the string
pub fn get_input_prefix(string: &str, count: usize) -> String {
    return string.chars().take(count).collect();
}
/// Get count number of characters from the end of the string
pub fn get_input_suffix(string: &str, count: usize) -> String {
    return string.chars().skip(count).collect();
}

// Insert the given character into the current prompt input
// (at current cursor position) by mutating prompt state
pub fn insert_char_into_input(state: &mut State<String>, config: &Config, c: char) {
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
            result.push_str(&util::get_input_prefix(&input, state.cursor_position));
            result.push_str(&c.to_string());
            result.push_str(&util::get_input_suffix(&input, state.cursor_position));
            state.input = Some(result);
        }
        None => {
            state.input = Some(c.to_string());
        }
    }
}

// Remove character at current cursor position from prompt input by mutating prompt state
pub fn remove_char_from_input(state: &mut State<String>, _config: &Config) {
    if let Some(input) = state.input.clone() {
        let mut result = String::from("");
        result.push_str(&util::get_input_prefix(
            &input,
            state.cursor_position.saturating_sub(1),
        ));
        result.push_str(&util::get_input_suffix(&input, state.cursor_position));
        state.input = match result.is_empty() {
            false => Some(result),
            true => None,
        };
    }
}

// Truncate string to given width, scrolling if necessary
pub fn truncate(s: String, width: usize, cursor_position: usize) -> String {
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

        result
    } else {
        s.chars().take(width).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(truncate("test".into(), 2, 0), "te".to_string());
    }

    #[test]
    fn t_truncate_x_scroll_middle() {
        assert_eq!(truncate("test123".into(), 2, 5), "t1".to_string());
    }

    #[test]
    fn t_truncate_x_scroll_end() {
        assert_eq!(truncate("test123".into(), 2, 7), "23".to_string());
    }

    #[test]
    fn t_truncate_x_normal_ellipsis() {
        // Doesn't properly respect width but best I can do right now
        assert_eq!(truncate("test123".into(), 4, 0), "test…".to_string());
    }

    #[test]
    fn t_truncate_x_scroll_middle_ellipsis() {
        assert_eq!(truncate("test123".into(), 4, 5), "…st1…".to_string());
    }

    #[test]
    fn t_truncate_x_scroll_end_ellipsis() {
        assert_eq!(truncate("test123".into(), 4, 7), "…123".to_string());
    }
}
