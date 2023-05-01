use anyhow::Result;
use crossterm::event;

mod types;

#[non_exhaustive]
#[derive(Debug)]
pub enum Role {
    Inactive,
    Active,
    Completed,
    Aborted,
}

#[non_exhaustive]
#[derive(Debug, Copy, Clone)]
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
    let terminal_width = usize::from(crossterm::terminal::size()?.0);
    Ok(match config.max_display_width {
        Some(x) => x.min(terminal_width),
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
    fn validate(state: &State<Self>) -> Status {
        state.status
    }
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
