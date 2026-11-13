mod pieces;
mod solver;

use crate::model::{Board, Direction, Pos, Tile};
use rand::{seq::SliceRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub const ALL: [Self; 3] = [Self::Easy, Self::Normal, Self::Hard];
    pub fn slide_range(self) -> std::ops::RangeInclusive<usize> {
        match self {
            Self::Easy => 3..=6,
            Self::Normal => 7..=11,
            Self::Hard => 12..=24,
        }
    }

    fn room_layout(self) -> (usize, usize, usize, usize) {
        match self {
            Self::Easy => (2, 1, 9, 7),
            Self::Normal => (2, 2, 9, 7),
            Self::Hard => (2, 2, 11, 7),
        }
    }
}

impl fmt::Display for Difficulty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
        })
    }
}

impl std::str::FromStr for Difficulty {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "easy" => Ok(Self::Easy),
            "normal" => Ok(Self::Normal),
            "hard" => Ok(Self::Hard),
            _ => Err("difficulty must be easy, normal, or hard".into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pars {
    pub any_percent_minimum: usize,
    pub full_clear_minimum: usize,
}

impl Pars {
    pub fn any_percent(&self) -> usize {
        Self::with_allowance(self.any_percent_minimum)
    }

    pub fn full_clear(&self) -> usize {
        Self::with_allowance(self.full_clear_minimum)
    }

    fn with_allowance(minimum: usize) -> usize {
        minimum + minimum.div_ceil(4).max(2)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stage {
    pub board: Board,
    pub start: Pos,
    pub seed: u64,
    pub difficulty: Difficulty,
    pub pars: Pars,
}

pub fn generate(seed: u64, difficulty: Difficulty) -> Result<Stage, String> {
    generate_with_budget(seed, difficulty, 100)
}

fn generate_with_budget(
    seed: u64,
    difficulty: Difficulty,
    attempts: usize,
) -> Result<Stage, String> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    for _ in 0..attempts {
        let assembled = assemble(difficulty, &mut rng);
        let board = &assembled;
        let mut floors: Vec<_> = (0..board.height)
            .flat_map(|y| (0..board.width).map(move |x| Pos { x, y }))
            .filter(|p| board.get(*p) == Tile::Floor)
            .collect();
        floors.shuffle(&mut rng);
        for start in floors.iter().take(12).copied() {
            let analysis = solver::analyze(board, start);
            let exits: Vec<_> = floors
                .iter()
                .copied()
                .filter(|p| {
                    start.x.abs_diff(p.x) * 2 >= board.width - 2
                        && start.y.abs_diff(p.y) * 2 >= board.height - 2
                        && analysis.distances[p.y * board.width + p.x]
                            .is_some_and(|d| difficulty.slide_range().contains(&d))
                })
                .collect();
            for exit in exits.into_iter().take(8) {
                let mut board = assembled.clone();
                board.set(exit, Tile::Exit);
                let verified = solver::analyze(&board, start);
                let Some(ref path) = verified.solution else {
                    continue;
                };
                if !difficulty.slide_range().contains(&path.len())
                    || !verified.hazards_allow_safe_waits(&board, start)
                    || board.tiles.iter().enumerate().any(|(index, tile)| {
                        matches!(
                            tile,
                            Tile::Floor
                                | Tile::Dot
                                | Tile::Coin
                                | Tile::TimedSpike
                                | Tile::TouchTrap
                        ) && !verified.safe_cells.contains(&Pos {
                            x: index % board.width,
                            y: index / board.width,
                        })
                    })
                {
                    continue;
                }
                let pickups: Vec<_> = floors
                    .iter()
                    .copied()
                    .filter(|pos| {
                        *pos != start
                            && board.get(*pos) == Tile::Floor
                            && verified.safe_stops.contains(pos)
                    })
                    .collect();
                let Some(&pickup) = pickups.choose(&mut rng) else {
                    continue;
                };
                board.set(pickup, Tile::Shield);
                // Inventory changes reachable routes, stopping cells, and exact
                // minimums. Prove the final board, not just its unshielded form.
                let verified = solver::analyze(&board, start);
                let Some(ref path) = verified.solution else {
                    continue;
                };
                if !difficulty.slide_range().contains(&path.len())
                    || !verified.hazards_allow_safe_waits(&board, start)
                {
                    continue;
                }
                // Iterate in board order, never HashSet order, to preserve seeded replay.
                let mut reward_number = rng.gen_range(0..10);
                for y in 1..board.height - 1 {
                    for x in 1..board.width - 1 {
                        let pos = Pos { x, y };
                        if pos != start
                            && board.get(pos) == Tile::Floor
                            && verified.safe_cells.contains(&pos)
                        {
                            board.set(
                                pos,
                                if reward_number % 11 == 0 {
                                    Tile::Coin
                                } else {
                                    Tile::Dot
                                },
                            );
                            reward_number += 1;
                        }
                    }
                }
                let Ok(full_path) = solver::full_clear(&board, start, &verified, 50_000) else {
                    continue;
                };
                return Ok(Stage {
                    board,
                    start,
                    seed,
                    difficulty,
                    pars: Pars {
                        any_percent_minimum: path.len(),
                        full_clear_minimum: full_path.len(),
                    },
                });
            }
        }
    }
    Err(format!(
        "Could not verify a {difficulty} stage for seed {seed}. Retry or choose another seed."
    ))
}

fn assemble(difficulty: Difficulty, rng: &mut impl Rng) -> Board {
    use pieces::Family;
    let (columns, rows, room_w, room_h) = difficulty.room_layout();
    let mut board = Board::filled(
        columns * (room_w + 1) + 1,
        rows * (room_h + 1) + 1,
        Tile::Wall,
    );
    let count = columns * rows;
    let mut families = vec![Family::MixedCrossing];
    if count > 2 {
        families.push(Family::TouchRun);
    }
    let mut transports = [
        Family::Switchback,
        Family::IslandReturn,
        Family::LoopChain,
        Family::RewardPocket,
    ];
    transports.shuffle(rng);
    families.extend(transports.into_iter().take(count - families.len()));
    families.shuffle(rng);
    let mut pieces = Vec::new();
    for row in 0..rows {
        for column in 0..columns {
            let mut required = Vec::new();
            if column > 0 {
                required.push(Direction::Left);
            }
            if column + 1 < columns {
                required.push(Direction::Right);
            }
            if row > 0 {
                required.push(Direction::Up);
            }
            if row + 1 < rows {
                required.push(Direction::Down);
            }
            let piece = pieces::make(
                room_w,
                room_h,
                families[row * columns + column],
                &required,
                rng,
            );
            let origin = Pos {
                x: 1 + column * (room_w + 1),
                y: 1 + row * (room_h + 1),
            };
            for y in 0..room_h {
                for x in 0..room_w {
                    board.set(
                        Pos {
                            x: origin.x + x,
                            y: origin.y + y,
                        },
                        piece.board.get(Pos { x, y }),
                    );
                }
            }
            pieces.push(piece);
        }
    }
    let port = |column: usize, row: usize, side: Direction| {
        let local = pieces[row * columns + column]
            .ports
            .iter()
            .find(|p| p.direction == side)
            .expect("selected template has the required port")
            .pos;
        Pos {
            x: 1 + column * (room_w + 1) + local.x,
            y: 1 + row * (room_h + 1) + local.y,
        }
    };
    for row in 0..rows {
        for column in 0..columns {
            // A spanning tree connects every piece; optional joins create loops.
            let join_left = column > 0 && (row == 0 || rng.gen_bool(0.5));
            let join_up = row > 0 && !join_left;
            if column > 0 && (join_left || rng.gen_bool(0.25)) {
                connect(
                    &mut board,
                    port(column - 1, row, Direction::Right),
                    port(column, row, Direction::Left),
                    true,
                );
            }
            if row > 0 && (join_up || rng.gen_bool(0.25)) {
                connect(
                    &mut board,
                    port(column, row - 1, Direction::Down),
                    port(column, row, Direction::Up),
                    false,
                );
            }
        }
    }
    board
}

fn connect(board: &mut Board, from: Pos, to: Pos, horizontal: bool) {
    // Only the divider is mutable. Port stubs and every template wall remain intact.
    if horizontal {
        let x = (from.x + to.x) / 2;
        for y in from.y.min(to.y)..=from.y.max(to.y) {
            board.set(Pos { x, y }, Tile::Floor);
        }
    } else {
        let y = (from.y + to.y) / 2;
        for x in from.x.min(to.x)..=from.x.max(to.x) {
            board.set(Pos { x, y }, Tile::Floor);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pars_allow_detours_and_round_fractional_allowances_up() {
        for (any_minimum, full_minimum, any_par, full_par) in [
            (3, 14, 5, 18),
            (8, 9, 10, 12),
            (12, 17, 15, 22),
            (20, 28, 25, 35),
        ] {
            let pars = Pars {
                any_percent_minimum: any_minimum,
                full_clear_minimum: full_minimum,
            };
            assert_eq!(pars.any_percent(), any_par);
            assert_eq!(pars.full_clear(), full_par);
        }
    }
    #[test]
    fn both_pars_have_playable_witnesses_and_full_clear_collects_every_reward() {
        for difficulty in Difficulty::ALL {
            for seed in 0..10 {
                let stage = generate(seed, difficulty).unwrap();
                assert_eq!(
                    stage
                        .board
                        .tiles
                        .iter()
                        .filter(|tile| **tile == Tile::Shield)
                        .count(),
                    1
                );
                let analysis = solver::analyze(&stage.board, stage.start);
                assert!(analysis.hazards_allow_safe_waits(&stage.board, stage.start));
                let path = solver::full_clear(&stage.board, stage.start, &analysis, 50_000)
                    .unwrap_or_else(|error| panic!("{difficulty} seed {seed}: {error:?}"));
                assert_eq!(
                    stage.pars.any_percent_minimum,
                    analysis.solution.as_ref().unwrap().len()
                );
                assert_eq!(
                    stage.pars.full_clear_minimum,
                    path.len(),
                    "{difficulty} seed {seed}"
                );
                assert!(stage.pars.full_clear_minimum >= stage.pars.any_percent_minimum);
                let mut game = crate::model::Game::new(stage.board, stage.start);
                for direction in path {
                    use crate::model::SpikePhase;
                    for _ in 0..70 {
                        game.tick();
                    }
                    for _ in 0..100 {
                        let before = game.spike_phase();
                        game.tick();
                        if before == SpikePhase::Active
                            && game.spike_phase() == SpikePhase::Retracted
                        {
                            break;
                        }
                    }
                    assert_eq!(game.outcome, crate::model::Outcome::Playing);
                    game.launch(direction);
                    while game.direction.is_some() {
                        game.tick();
                    }
                }
                assert_eq!(game.outcome, crate::model::Outcome::Cleared);
                assert_eq!(game.slides as usize, stage.pars.full_clear_minimum);
                assert!(!game
                    .board
                    .tiles
                    .iter()
                    .any(|tile| matches!(tile, Tile::Dot | Tile::Coin)));
            }
        }
    }
    #[test]
    fn connectors_preserve_every_template_cell_in_both_orientations() {
        for horizontal in [true, false] {
            let mut board = Board::filled(9, 9, Tile::Coin);
            for offset in 0..9 {
                board.set(
                    if horizontal {
                        Pos { x: 4, y: offset }
                    } else {
                        Pos { x: offset, y: 4 }
                    },
                    Tile::Wall,
                );
            }
            let original = board.clone();
            let (from, to) = if horizontal {
                (Pos { x: 3, y: 2 }, Pos { x: 5, y: 6 })
            } else {
                (Pos { x: 2, y: 3 }, Pos { x: 6, y: 5 })
            };
            connect(&mut board, from, to, horizontal);
            for y in 0..9 {
                for x in 0..9 {
                    let pos = Pos { x, y };
                    let (divider, along) = if horizontal { (x == 4, y) } else { (y == 4, x) };
                    assert_eq!(
                        board.get(pos),
                        if divider && (2..=6).contains(&along) {
                            Tile::Floor
                        } else {
                            original.get(pos)
                        }
                    );
                }
            }
        }
    }
    #[test]
    fn exhausted_budget_returns_replayable_error() {
        let error = generate_with_budget(42, Difficulty::Hard, 0).unwrap_err();
        assert!(error.contains("42") && error.contains("hard"));
    }

    #[test]
    fn generated_shields_can_open_fixed_spike_shortcuts() {
        let mut shortcuts = 0;
        for difficulty in Difficulty::ALL {
            for seed in 0..20 {
                let stage = generate(seed, difficulty).unwrap();
                let mut unshielded = stage.board.clone();
                for tile in &mut unshielded.tiles {
                    if *tile == Tile::Shield {
                        *tile = Tile::Floor;
                    }
                }
                let baseline = solver::analyze(&unshielded, stage.start)
                    .solution
                    .unwrap()
                    .len();
                if stage.pars.any_percent_minimum < baseline {
                    shortcuts += 1;
                }
            }
        }
        assert!(
            shortcuts > 0,
            "generated shields should enable real shortcuts"
        );
    }
}
