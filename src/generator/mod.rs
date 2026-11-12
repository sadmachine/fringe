mod pieces;
mod solver;

use crate::model::{Board, Pos, Tile};
use rand::{seq::SliceRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::{collections::HashSet, fmt};

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
            Self::Easy => (2, 1, 7, 5),
            Self::Normal => (2, 2, 7, 5),
            Self::Hard => (2, 2, 9, 7),
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
            let reachable: HashSet<_> = floors
                .iter()
                .copied()
                .filter(|p| analysis.distances[p.y * board.width + p.x].is_some())
                .collect();
            if !useful_rooms(board, &reachable, difficulty) {
                continue;
            }
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
                let reachable = solver::analyze(&board, start);
                if !useful_rooms(&board, &reachable.safe_cells, difficulty) {
                    continue;
                }
                // Small unplayable pockets become solid decoration. Reject whole unused rooms above.
                for pos in &floors {
                    if board.get(*pos) == Tile::Floor && !reachable.safe_cells.contains(pos) {
                        board.set(*pos, Tile::Wall);
                    }
                }
                let check = solver::analyze(&board, start);
                if !check
                    .solution
                    .as_ref()
                    .is_some_and(|path| difficulty.slide_range().contains(&path.len()))
                {
                    continue;
                }
                let spike_target = match difficulty {
                    Difficulty::Easy => 1,
                    Difficulty::Normal => 4,
                    Difficulty::Hard => 9,
                };
                let mut placed = 0;
                for candidate in floors
                    .iter()
                    .filter(|p| **p != start && **p != exit)
                    .take(spike_target * 5)
                {
                    if board.get(*candidate) != Tile::Floor {
                        continue;
                    }
                    board.set(*candidate, Tile::Spike);
                    let check = solver::analyze(&board, start);
                    if check
                        .solution
                        .as_ref()
                        .is_some_and(|p| difficulty.slide_range().contains(&p.len()))
                        && floors
                            .iter()
                            .all(|p| board.get(*p) != Tile::Floor || check.safe_cells.contains(p))
                    {
                        placed += 1;
                    } else {
                        board.set(*candidate, Tile::Floor);
                    }
                    if placed == spike_target {
                        break;
                    }
                }
                let verified = solver::analyze(&board, start);
                let Some(ref path) = verified.solution else {
                    continue;
                };
                if !difficulty.slide_range().contains(&path.len()) {
                    continue;
                }
                if floors
                    .iter()
                    .any(|p| board.get(*p) == Tile::Floor && !verified.safe_cells.contains(p))
                {
                    continue;
                }
                // Iterate in board order, never HashSet order, to preserve seeded replay.
                let mut reward_number = rng.gen_range(0..10);
                for y in 1..board.height - 1 {
                    for x in 1..board.width - 1 {
                        let pos = Pos { x, y };
                        if pos != start && verified.safe_cells.contains(&pos) {
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

fn useful_rooms(board: &Board, reachable: &HashSet<Pos>, difficulty: Difficulty) -> bool {
    let (columns, rows, room_w, room_h) = difficulty.room_layout();
    for row in 0..rows {
        for column in 0..columns {
            let mut total = 0;
            let mut usable = 0;
            for y in 1 + row * (room_h + 1)..1 + row * (room_h + 1) + room_h {
                for x in 1 + column * (room_w + 1)..1 + column * (room_w + 1) + room_w {
                    let pos = Pos { x, y };
                    if board.get(pos) == Tile::Floor {
                        total += 1;
                        usable += usize::from(reachable.contains(&pos));
                    }
                }
            }
            if total == 0 || usable * 4 < total * 3 {
                return false;
            }
        }
    }
    true
}

fn assemble(difficulty: Difficulty, rng: &mut impl Rng) -> Board {
    use pieces::Family;
    let (columns, rows, room_w, room_h) = difficulty.room_layout();
    let width = columns * (room_w + 1) + 1;
    let height = rows * (room_h + 1) + 1;
    let mut board = Board::filled(width, height, Tile::Wall);
    let mut families = vec![Family::Switchback, Family::Loop, Family::Chamber];
    while families.len() < columns * rows {
        families.push(if rng.gen_bool(0.65) {
            Family::Switchback
        } else {
            Family::Loop
        });
    }
    families.shuffle(rng);
    for row in 0..rows {
        for column in 0..columns {
            let piece = pieces::make(room_w, room_h, families[row * columns + column], rng);
            for y in 0..room_h {
                for x in 0..room_w {
                    board.set(
                        Pos {
                            x: 1 + column * (room_w + 1) + x,
                            y: 1 + row * (room_h + 1) + y,
                        },
                        piece.get(Pos { x, y }),
                    );
                }
            }
        }
    }
    for row in 0..rows {
        for column in 0..columns {
            // Every region joins a previous region, forming a spanning tree.
            let join_left = column > 0 && (row == 0 || rng.gen_bool(0.5));
            let join_up = row > 0 && !join_left;
            if column > 0 && (join_left || rng.gen_bool(0.25)) {
                let port = Pos {
                    x: column * (room_w + 1),
                    y: 1 + row * (room_h + 1) + rng.gen_range(0..room_h),
                };
                connect(&mut board, port, (1, 0), room_w, room_h);
            }
            if row > 0 && (join_up || rng.gen_bool(0.25)) {
                let port = Pos {
                    x: 1 + column * (room_w + 1) + rng.gen_range(0..room_w),
                    y: row * (room_h + 1),
                };
                connect(&mut board, port, (0, 1), room_w, room_h);
            }
        }
    }
    board
}

fn connect(board: &mut Board, port: Pos, (dx, dy): (isize, isize), room_w: usize, room_h: usize) {
    board.set(port, Tile::Floor);
    for sign in [-1, 1] {
        let mut pos = Pos {
            x: port.x.checked_add_signed(dx * sign).unwrap(),
            y: port.y.checked_add_signed(dy * sign).unwrap(),
        };
        let left = 1 + (pos.x - 1) / (room_w + 1) * (room_w + 1);
        let top = 1 + (pos.y - 1) / (room_h + 1) * (room_h + 1);
        let target = (top..top + room_h)
            .flat_map(|y| (left..left + room_w).map(move |x| Pos { x, y }))
            .filter(|p| board.get(*p) == Tile::Floor)
            .min_by_key(|p| p.x.abs_diff(pos.x) + p.y.abs_diff(pos.y))
            .expect("a generated piece has floor");
        board.set(pos, Tile::Floor);
        // Carve only inside the neighboring piece, even if the port faces a solid edge.
        while pos != target {
            if pos.x != target.x {
                pos.x = pos
                    .x
                    .checked_add_signed(if pos.x < target.x { 1 } else { -1 })
                    .unwrap();
            } else {
                pos.y = pos
                    .y
                    .checked_add_signed(if pos.y < target.y { 1 } else { -1 })
                    .unwrap();
            }
            board.set(pos, Tile::Floor);
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
                let analysis = solver::analyze(&stage.board, stage.start);
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
    fn useful_rooms_rejects_a_missing_section_but_allows_small_pockets() {
        let mut board = Board::filled(17, 13, Tile::Wall);
        let mut floors = HashSet::new();
        for top in [1, 7] {
            for left in [1, 9] {
                for y in top..top + 5 {
                    for x in left..left + 7 {
                        let pos = Pos { x, y };
                        board.set(pos, Tile::Floor);
                        floors.insert(pos);
                    }
                }
            }
        }
        let mut reachable = floors.clone();
        reachable.retain(|p| p.x >= 9 || p.y >= 7);
        assert!(!useful_rooms(&board, &reachable, Difficulty::Normal));
        for pos in [
            Pos { x: 1, y: 1 },
            Pos { x: 9, y: 1 },
            Pos { x: 1, y: 7 },
            Pos { x: 9, y: 7 },
        ] {
            floors.remove(&pos);
        }
        assert!(useful_rooms(&board, &floors, Difficulty::Normal));
    }
    #[test]
    fn exhausted_budget_returns_replayable_error() {
        let error = generate_with_budget(42, Difficulty::Hard, 0).unwrap_err();
        assert!(error.contains("42") && error.contains("hard"));
    }
}
