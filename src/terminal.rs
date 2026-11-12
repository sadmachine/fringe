use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    style::ResetColor,
    terminal::{self, EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen},
};
use fringe::{
    app::{Action, App, Screen},
    generator::Difficulty,
    model::Direction,
};
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub const MIN_SIZE: (u16, u16) = (80, 24);
static ACTIVE: AtomicBool = AtomicBool::new(false);

fn restore() {
    if ACTIVE.swap(false, Ordering::SeqCst) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            EndSynchronizedUpdate,
            ResetColor,
            Show,
            LeaveAlternateScreen
        );
    }
}

struct Session;
impl Session {
    fn enter() -> io::Result<Self> {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            previous(info);
        }));
        terminal::enable_raw_mode()?;
        ACTIVE.store(true, Ordering::SeqCst);
        let session = Self;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(session)
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        restore();
    }
}

pub fn run(app: &mut App) -> io::Result<()> {
    let _session = Session::enter()?;
    let mut out = io::stdout();
    let mut size = terminal::size()?;
    let mut last = Instant::now();
    let mut clock = Clock::default();
    let mut dirty = true;
    while !app.quit {
        if dirty {
            crate::render::draw(&mut out, app, size)?;
            dirty = false;
        }
        if event::poll(Duration::from_millis(8))? {
            for _ in 0..32 {
                match event::read()? {
                    Event::Resize(width, height) => {
                        size = (width, height);
                        dirty = true;
                    }
                    Event::Key(key) if key.kind != KeyEventKind::Release => {
                        if let Some(action) = action(key) {
                            if fits(size) || matches!(action, Action::Quit) {
                                app.handle(action);
                                dirty = true;
                            }
                        }
                    }
                    _ => {}
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
        let now = Instant::now();
        let elapsed = now.duration_since(last);
        last = now;
        for _ in 0..clock.advance(elapsed, fits(size) && app.running()) {
            let moving =
                matches!(&app.screen, Screen::Playing(run) if run.game.direction.is_some());
            app.tick();
            dirty |= moving;
        }
    }
    Ok(())
}

pub fn fits(size: (u16, u16)) -> bool {
    size.0 >= MIN_SIZE.0 && size.1 >= MIN_SIZE.1
}

fn action(key: KeyEvent) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Action::Quit);
    }
    Some(match key.code {
        KeyCode::Up => Action::Move(Direction::Up),
        KeyCode::Right => Action::Move(Direction::Right),
        KeyCode::Down => Action::Move(Direction::Down),
        KeyCode::Left => Action::Move(Direction::Left),
        KeyCode::Enter => Action::Start,
        KeyCode::Esc => Action::Menu,
        KeyCode::Char(c) => match c.to_ascii_lowercase() {
            'w' | 'k' => Action::Move(Direction::Up),
            'd' | 'l' => Action::Move(Direction::Right),
            's' | 'j' => Action::Move(Direction::Down),
            'a' | 'h' => Action::Move(Direction::Left),
            '1' => Action::Choose(Difficulty::Easy),
            '2' => Action::Choose(Difficulty::Normal),
            '3' => Action::Choose(Difficulty::Hard),
            'p' | ' ' => Action::Pause,
            'r' => Action::Retry,
            'n' => Action::Next,
            'm' => Action::Menu,
            'q' => Action::Quit,
            _ => return None,
        },
        _ => return None,
    })
}

#[derive(Default)]
struct Clock {
    accumulated: Duration,
    was_running: bool,
}

impl Clock {
    fn advance(&mut self, elapsed: Duration, running: bool) -> usize {
        if !running || !self.was_running {
            self.accumulated = Duration::ZERO;
            self.was_running = running;
            return 0;
        }
        self.accumulated += elapsed.min(Duration::from_millis(120));
        let steps = (self.accumulated.as_millis() / 40).min(3) as usize;
        self.accumulated -= Duration::from_millis(40 * steps as u64);
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pause_and_resize_discard_time_and_stalls_have_bounded_catch_up() {
        let mut clock = Clock::default();
        assert_eq!(clock.advance(Duration::from_secs(10), true), 0);
        assert_eq!(clock.advance(Duration::from_millis(39), true), 0);
        assert_eq!(clock.advance(Duration::from_millis(1), true), 1);
        assert_eq!(clock.advance(Duration::from_secs(10), true), 3);
        assert_eq!(clock.advance(Duration::from_secs(10), false), 0);
        assert_eq!(clock.advance(Duration::from_secs(10), true), 0);
        assert_eq!(clock.advance(Duration::from_millis(39), true), 0);
    }
}
