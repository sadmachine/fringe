use crate::generator::{generate, Difficulty, Stage};
use crate::model::{Direction, Game, Outcome};

pub enum Action {
    Move(Direction),
    Choose(Difficulty),
    Start,
    Pause,
    Retry,
    Next,
    Menu,
    Quit,
}

pub struct Run {
    pub stage: Stage,
    pub game: Game,
    pub paused: bool,
}

pub enum Screen {
    Menu,
    Playing(Box<Run>),
    Error(String),
}

pub struct App {
    pub screen: Screen,
    pub difficulty: Difficulty,
    pub seed: u64,
    pub quit: bool,
}

impl App {
    pub fn new(seed: u64, difficulty: Difficulty) -> Self {
        Self {
            screen: Screen::Menu,
            difficulty,
            seed,
            quit: false,
        }
    }
    pub fn handle(&mut self, action: Action) {
        match action {
            Action::Quit => {
                self.quit = true;
                return;
            }
            Action::Menu => {
                self.screen = Screen::Menu;
                return;
            }
            _ => {}
        }
        match &mut self.screen {
            Screen::Menu => match action {
                Action::Choose(difficulty) => self.difficulty = difficulty,
                Action::Move(direction) => {
                    let index = Difficulty::ALL
                        .iter()
                        .position(|d| *d == self.difficulty)
                        .unwrap();
                    let delta = match direction {
                        Direction::Up | Direction::Left => 2,
                        _ => 1,
                    };
                    self.difficulty = Difficulty::ALL[(index + delta) % 3];
                }
                Action::Start => self.load(self.seed),
                _ => {}
            },
            Screen::Playing(run) => match action {
                Action::Retry => self.load(self.seed),
                Action::Next => self.load(self.seed.wrapping_add(1)),
                Action::Start if run.game.outcome != Outcome::Playing => {
                    self.load(self.seed.wrapping_add(1))
                }
                Action::Pause if run.game.outcome == Outcome::Playing => run.paused = !run.paused,
                Action::Move(direction) if !run.paused => run.game.launch(direction),
                _ => {}
            },
            Screen::Error(_) => {
                if matches!(action, Action::Next | Action::Start | Action::Retry) {
                    self.load(self.seed.wrapping_add(1));
                }
            }
        }
    }
    pub fn tick(&mut self) {
        if let Screen::Playing(run) = &mut self.screen {
            if !run.paused {
                run.game.tick();
            }
        }
    }
    pub fn running(&self) -> bool {
        matches!(&self.screen, Screen::Playing(run) if !run.paused && run.game.outcome == Outcome::Playing)
    }
    fn load(&mut self, seed: u64) {
        self.seed = seed;
        self.screen = match generate(seed, self.difficulty) {
            Ok(stage) => {
                let game = Game::new(stage.board.clone(), stage.start);
                Screen::Playing(Box::new(Run {
                    stage,
                    game,
                    paused: false,
                }))
            }
            Err(message) => Screen::Error(message),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing(app: &App) -> &Run {
        let Screen::Playing(run) = &app.screen else {
            panic!("expected stage");
        };
        run
    }

    #[test]
    fn menu_selects_difficulty_and_starts_a_replayable_stage() {
        let mut app = App::new(42, Difficulty::Normal);
        app.handle(Action::Move(Direction::Down));
        assert_eq!(app.difficulty, Difficulty::Hard);
        app.handle(Action::Choose(Difficulty::Easy));
        app.handle(Action::Start);
        assert_eq!(playing(&app).stage, generate(42, Difficulty::Easy).unwrap());
    }

    #[test]
    fn pause_freezes_a_slide_and_retry_restores_the_original_stage() {
        let mut app = App::new(42, Difficulty::Easy);
        app.handle(Action::Start);
        let original = playing(&app).stage.clone();
        let direction = Direction::ALL
            .into_iter()
            .find(|d| original.board.step(original.start, *d) != crate::model::Step::Blocked)
            .unwrap();
        app.handle(Action::Move(direction));
        app.handle(Action::Pause);
        assert!(!app.running());
        for _ in 0..20 {
            app.tick();
        }
        assert_eq!(playing(&app).game.player, original.start);
        app.handle(Action::Pause);
        app.tick();
        assert_ne!(playing(&app).game.player, original.start);
        app.handle(Action::Retry);
        assert_eq!(playing(&app).stage, original);
        assert_eq!(playing(&app).game.player, original.start);
        assert_eq!(playing(&app).game.score, 0);
        assert_eq!(playing(&app).game.slides, 0);
    }

    #[test]
    fn new_stage_advances_seed_and_error_screen_can_recover() {
        let mut app = App::new(u64::MAX, Difficulty::Normal);
        app.screen = Screen::Error("Generation failed".into());
        app.handle(Action::Next);
        assert_eq!(playing(&app).stage.seed, 0);
        app.handle(Action::Menu);
        assert!(matches!(app.screen, Screen::Menu));
        app.handle(Action::Quit);
        assert!(app.quit);
    }

    #[test]
    fn result_enter_starts_another_stage() {
        let mut app = App::new(7, Difficulty::Easy);
        app.handle(Action::Start);
        if let Screen::Playing(run) = &mut app.screen {
            run.game.outcome = Outcome::Cleared;
        }
        app.handle(Action::Start);
        assert_eq!(playing(&app).stage.seed, 8);
        assert_eq!(playing(&app).game.outcome, Outcome::Playing);
    }
}
