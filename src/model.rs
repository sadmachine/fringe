#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Wall,
    Floor,
    Dot,
    Coin,
    Spike,
    TimedSpike,
    TouchTrap,
    Exit,
}

// Simulation ticks are 40 ms. All timed walls share one four-second cycle.
pub(crate) const SPIKE_RETRACTED_TICKS: u16 = 60;
const SPIKE_WARNING_TICKS: u16 = 20;
const SPIKE_CYCLE_TICKS: u16 = 100;
pub(crate) const TOUCH_DELAY_TICKS: u16 = 50;
const TOUCH_ACTIVE_TICKS: u16 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpikePhase {
    Retracted,
    Warning,
    Active,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    pub const ALL: [Self; 4] = [Self::Up, Self::Right, Self::Down, Self::Left];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Tile>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Blocked,
    Moved(Pos),
    Died(Pos),
    Cleared(Pos),
}

impl Board {
    pub fn filled(width: usize, height: usize, tile: Tile) -> Self {
        Self {
            width,
            height,
            tiles: vec![tile; width * height],
        }
    }
    pub fn get(&self, pos: Pos) -> Tile {
        self.tiles[pos.y * self.width + pos.x]
    }
    pub fn set(&mut self, pos: Pos, tile: Tile) {
        self.tiles[pos.y * self.width + pos.x] = tile;
    }
    /// Geometric traversal. Game checks timed spike damage on simulation ticks.
    pub fn step(&self, pos: Pos, direction: Direction) -> Step {
        let (dx, dy) = match direction {
            Direction::Up => (0, -1),
            Direction::Right => (1, 0),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
        };
        let Some(x) = pos.x.checked_add_signed(dx) else {
            return Step::Blocked;
        };
        let Some(y) = pos.y.checked_add_signed(dy) else {
            return Step::Blocked;
        };
        if x >= self.width || y >= self.height {
            return Step::Blocked;
        }
        let next = Pos { x, y };
        match self.get(next) {
            Tile::Wall => Step::Blocked,
            Tile::Spike => Step::Died(next),
            Tile::Exit => Step::Cleared(next),
            _ => Step::Moved(next),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Playing,
    Died,
    Cleared,
}

struct TouchTrap {
    pos: Pos,
    // None is ready; contact starts a countdown that later contacts cannot reset.
    age: Option<u16>,
}

impl TouchTrap {
    fn phase(&self) -> SpikePhase {
        match self.age {
            None => SpikePhase::Retracted,
            Some(age) if age < TOUCH_DELAY_TICKS => SpikePhase::Warning,
            Some(_) => SpikePhase::Active,
        }
    }
}

pub struct Game {
    pub board: Board,
    pub player: Pos,
    pub direction: Option<Direction>,
    pub outcome: Outcome,
    pub score: u32,
    pub slides: u32,
    hazard_tick: u16,
    touch_traps: Vec<TouchTrap>,
}

impl Game {
    pub fn new(board: Board, start: Pos) -> Self {
        let touch_traps = board
            .tiles
            .iter()
            .enumerate()
            .filter_map(|(index, tile)| {
                (*tile == Tile::TouchTrap).then_some(TouchTrap {
                    pos: Pos {
                        x: index % board.width,
                        y: index / board.width,
                    },
                    age: None,
                })
            })
            .collect();
        Self {
            board,
            player: start,
            direction: None,
            outcome: Outcome::Playing,
            score: 0,
            slides: 0,
            hazard_tick: 0,
            touch_traps,
        }
    }
    pub fn spike_phase(&self) -> SpikePhase {
        if self.hazard_tick < SPIKE_RETRACTED_TICKS {
            SpikePhase::Retracted
        } else if self.hazard_tick < SPIKE_RETRACTED_TICKS + SPIKE_WARNING_TICKS {
            SpikePhase::Warning
        } else {
            SpikePhase::Active
        }
    }
    pub fn spike_phase_at(&self, pos: Pos) -> SpikePhase {
        match self.board.get(pos) {
            Tile::Spike => SpikePhase::Active,
            Tile::TimedSpike => self.spike_phase(),
            Tile::TouchTrap => self
                .touch_traps
                .iter()
                .find(|trap| trap.pos == pos)
                .expect("touch trap tile has a timer")
                .phase(),
            _ => SpikePhase::Retracted,
        }
    }
    pub fn launch(&mut self, direction: Direction) {
        if self.outcome == Outcome::Playing
            && self.direction.is_none()
            && self.board.step(self.player, direction) != Step::Blocked
        {
            self.direction = Some(direction);
            self.slides += 1;
        }
    }
    /// Advance movement and hazards together, reporting whether the view changed.
    pub fn tick(&mut self) -> bool {
        if self.outcome != Outcome::Playing {
            return false;
        }
        let previous_phase = self.spike_phase();
        self.hazard_tick = (self.hazard_tick + 1) % SPIKE_CYCLE_TICKS;
        let mut changed = previous_phase != self.spike_phase();
        for trap in &mut self.touch_traps {
            let previous = trap.phase();
            trap.age = trap.age.and_then(|age| {
                (age + 1 < TOUCH_DELAY_TICKS + TOUCH_ACTIVE_TICKS).then_some(age + 1)
            });
            changed |= previous != trap.phase();
        }
        // Activation happens before movement, including while standing still.
        if self.spike_phase_at(self.player) == SpikePhase::Active {
            self.outcome = Outcome::Died;
            self.direction = None;
            return true;
        }
        let Some(direction) = self.direction else {
            return changed;
        };
        match self.board.step(self.player, direction) {
            Step::Blocked => {
                self.direction = None;
                return changed;
            }
            Step::Moved(pos) => {
                self.player = pos;
                let tile = self.board.get(pos);
                self.score += match tile {
                    Tile::Dot => 1,
                    Tile::Coin => 10,
                    _ => 0,
                };
                if matches!(tile, Tile::Dot | Tile::Coin) {
                    self.board.set(pos, Tile::Floor);
                }
                if tile == Tile::TouchTrap {
                    self.touch_traps
                        .iter_mut()
                        .find(|trap| trap.pos == pos)
                        .expect("touch trap tile has a timer")
                        .age
                        .get_or_insert(0);
                }
                if self.spike_phase_at(pos) == SpikePhase::Active {
                    self.outcome = Outcome::Died;
                }
            }
            Step::Died(pos) => {
                self.player = pos;
                self.outcome = Outcome::Died;
            }
            Step::Cleared(pos) => {
                self.player = pos;
                self.outcome = Outcome::Cleared;
            }
        }
        if self.outcome != Outcome::Playing
            || self.board.step(self.player, direction) == Step::Blocked
        {
            self.direction = None;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corridor(tiles: &[Tile]) -> Game {
        let mut board = Board::filled(tiles.len() + 2, 3, Tile::Wall);
        for (x, tile) in tiles.iter().enumerate() {
            board.set(Pos { x: x + 1, y: 1 }, *tile);
        }
        Game::new(board, Pos { x: 1, y: 1 })
    }

    #[test]
    fn slide_advances_one_cell_and_stops_at_wall() {
        let mut game = corridor(&[Tile::Floor, Tile::Floor, Tile::Floor]);
        game.launch(Direction::Right);
        assert_eq!(game.player, Pos { x: 1, y: 1 });
        game.tick();
        assert_eq!(game.player, Pos { x: 2, y: 1 });
        game.tick();
        assert_eq!(game.player, Pos { x: 3, y: 1 });
        assert_eq!(game.direction, None);
        assert_eq!(game.slides, 1);
    }

    #[test]
    fn directions_during_a_slide_are_ignored() {
        let mut game = corridor(&[Tile::Floor, Tile::Floor, Tile::Floor]);
        game.launch(Direction::Right);
        game.launch(Direction::Left);
        game.tick();
        assert_eq!(game.player.x, 2);
        assert_eq!(game.slides, 1);
    }

    #[test]
    fn collects_intermediate_tiles_only_once() {
        let mut game = corridor(&[Tile::Floor, Tile::Dot, Tile::Coin]);
        game.launch(Direction::Right);
        game.tick();
        game.tick();
        assert_eq!(game.score, 11);
        game.launch(Direction::Left);
        game.tick();
        game.tick();
        assert_eq!(game.score, 11);
        assert_eq!(game.board.get(Pos { x: 2, y: 1 }), Tile::Floor);
    }

    #[test]
    fn spike_kills_in_transit_and_prevents_further_movement() {
        let mut game = corridor(&[Tile::Floor, Tile::Spike, Tile::Coin]);
        game.launch(Direction::Right);
        game.tick();
        assert_eq!(game.outcome, Outcome::Died);
        assert_eq!(game.player.x, 2);
        game.launch(Direction::Right);
        game.tick();
        assert_eq!(game.player.x, 2);
        assert_eq!(game.score, 0);
    }

    #[test]
    fn exit_clears_in_transit_without_collecting_every_dot() {
        let mut game = corridor(&[Tile::Floor, Tile::Exit, Tile::Dot]);
        game.launch(Direction::Right);
        game.tick();
        assert_eq!(game.outcome, Outcome::Cleared);
        assert_eq!(game.player.x, 2);
        assert_eq!(game.direction, None);
    }

    #[test]
    fn blocked_commands_do_not_count_as_slides_and_bounds_are_walls() {
        let mut game = corridor(&[Tile::Floor]);
        game.launch(Direction::Up);
        assert_eq!(game.slides, 0);
        assert_eq!(game.direction, None);
        let board = Board::filled(1, 1, Tile::Floor);
        for direction in Direction::ALL {
            assert_eq!(board.step(Pos { x: 0, y: 0 }, direction), Step::Blocked);
        }
    }

    #[test]
    fn timed_spikes_warn_then_activate_while_the_player_waits() {
        let mut game = corridor(&[Tile::Floor, Tile::TimedSpike, Tile::Floor]);
        for _ in 0..59 {
            game.tick();
        }
        assert_eq!(game.spike_phase(), SpikePhase::Retracted);
        game.tick();
        assert_eq!(game.spike_phase(), SpikePhase::Warning);
        for _ in 0..20 {
            game.tick();
        }
        assert_eq!(game.spike_phase(), SpikePhase::Active);
        game.launch(Direction::Right);
        game.tick();
        assert_eq!(game.outcome, Outcome::Died);
        assert_eq!(game.player.x, 2);
        let phase = game.spike_phase();
        for _ in 0..100 {
            game.tick();
        }
        assert_eq!(game.spike_phase(), phase, "death freezes the hazard cycle");
    }

    #[test]
    fn retracted_and_warning_spikes_are_passable_and_never_collected() {
        for tick in [0, 60] {
            let mut game = corridor(&[Tile::Floor, Tile::TimedSpike, Tile::Coin]);
            for _ in 0..tick {
                game.tick();
            }
            game.launch(Direction::Right);
            game.tick();
            game.tick();
            assert_eq!(game.outcome, Outcome::Playing);
            assert_eq!(game.player.x, 3);
            assert_eq!(game.score, 10);
            assert_eq!(game.board.get(Pos { x: 2, y: 1 }), Tile::TimedSpike);
        }
    }

    #[test]
    fn activation_kills_an_occupant_before_they_can_move_away() {
        let mut game = corridor(&[Tile::Floor, Tile::TimedSpike, Tile::Floor]);
        game.launch(Direction::Right);
        game.tick();
        game.direction = None;
        for _ in 1..79 {
            game.tick();
        }
        assert_eq!(game.outcome, Outcome::Playing);
        game.launch(Direction::Right);
        game.tick();
        assert_eq!(game.outcome, Outcome::Died);
        assert_eq!(game.player.x, 2);
        assert_eq!(game.direction, None);
    }

    #[test]
    fn waiting_for_retraction_costs_no_slides() {
        let mut game = corridor(&[Tile::Floor, Tile::TimedSpike, Tile::Exit]);
        for _ in 0..100 {
            game.tick();
        }
        assert_eq!(game.spike_phase(), SpikePhase::Retracted);
        assert_eq!(game.slides, 0);
        game.launch(Direction::Right);
        game.tick();
        game.tick();
        assert_eq!(game.outcome, Outcome::Cleared);
        assert_eq!(game.slides, 1);
    }

    #[test]
    fn touch_traps_wait_for_contact_then_warn_fire_and_reset() {
        let trap = Pos { x: 2, y: 1 };
        let mut game = corridor(&[Tile::Floor, Tile::TouchTrap, Tile::Floor]);
        for _ in 0..150 {
            game.tick();
        }
        assert_eq!(game.spike_phase_at(trap), SpikePhase::Retracted);
        game.launch(Direction::Right);
        game.tick();
        assert_eq!(game.spike_phase_at(trap), SpikePhase::Warning);
        game.tick();
        for _ in 0..48 {
            game.tick();
        }
        assert_eq!(game.spike_phase_at(trap), SpikePhase::Warning);
        assert!(game.tick(), "activation must redraw even while waiting");
        assert_eq!(game.spike_phase_at(trap), SpikePhase::Active);
        for _ in 0..20 {
            game.tick();
        }
        assert_eq!(game.spike_phase_at(trap), SpikePhase::Retracted);
        assert_eq!(game.board.get(trap), Tile::TouchTrap);
        assert_eq!(game.outcome, Outcome::Playing);
        assert_eq!(game.slides, 1);
        game.launch(Direction::Left);
        game.tick();
        assert_eq!(game.spike_phase_at(trap), SpikePhase::Warning);
    }

    #[test]
    fn crossing_an_armed_touch_trap_does_not_restart_its_countdown() {
        let trap = Pos { x: 2, y: 1 };
        let mut game = corridor(&[Tile::Floor, Tile::TouchTrap, Tile::Floor]);
        for direction in [Direction::Right, Direction::Left] {
            game.launch(direction);
            game.tick();
            game.tick();
        }
        for _ in 0..46 {
            game.tick();
        }
        assert_eq!(game.spike_phase_at(trap), SpikePhase::Warning);
        game.tick();
        assert_eq!(game.spike_phase_at(trap), SpikePhase::Active);
        assert_eq!(game.outcome, Outcome::Playing);
        game.launch(Direction::Right);
        game.tick();
        assert_eq!(game.outcome, Outcome::Died);
        assert_eq!(game.player, trap);
        let phase = game.spike_phase_at(trap);
        for _ in 0..100 {
            game.tick();
        }
        assert_eq!(
            game.spike_phase_at(trap),
            phase,
            "death freezes touch traps"
        );
    }

    #[test]
    fn touch_trap_activation_kills_a_stationary_occupant() {
        let mut game = corridor(&[Tile::Floor, Tile::TouchTrap, Tile::Floor]);
        game.launch(Direction::Right);
        game.tick();
        game.direction = None;
        for _ in 0..49 {
            game.tick();
        }
        assert_eq!(game.outcome, Outcome::Playing);
        game.tick();
        assert_eq!(game.outcome, Outcome::Died);
        assert_eq!(game.player, Pos { x: 2, y: 1 });
    }

    #[test]
    fn touch_traps_have_independent_countdowns() {
        let mut game = corridor(&[Tile::Floor, Tile::TouchTrap, Tile::TouchTrap, Tile::Floor]);
        game.launch(Direction::Right);
        game.tick();
        game.tick();
        game.tick();
        for _ in 0..48 {
            game.tick();
        }
        assert_eq!(game.spike_phase_at(Pos { x: 2, y: 1 }), SpikePhase::Active);
        assert_eq!(game.spike_phase_at(Pos { x: 3, y: 1 }), SpikePhase::Warning);
        game.tick();
        assert_eq!(game.spike_phase_at(Pos { x: 3, y: 1 }), SpikePhase::Active);
    }
}
