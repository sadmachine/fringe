mod pieces;
mod solver;

use crate::model::{Board, Pos, Tile};
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
pub struct Stage {
    pub board: Board,
    pub start: Pos,
    pub seed: u64,
    pub difficulty: Difficulty,
    pub par: usize,
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
        let mut board = assemble(difficulty, &mut rng);
        let mut floors: Vec<_> = (0..board.height)
            .flat_map(|y| (0..board.width).map(move |x| Pos { x, y }))
            .filter(|p| board.get(*p) == Tile::Floor)
            .collect();
        floors.shuffle(&mut rng);
        for start in floors.iter().take(12).copied() {
            let analysis = solver::analyze(&board, start);
            let exits: Vec<_> = floors
                .iter()
                .copied()
                .filter(|p| {
                    analysis.distances[p.y * board.width + p.x]
                        .is_some_and(|d| difficulty.slide_range().contains(&d))
                })
                .collect();
            let Some(exit) = exits.choose(&mut rng).copied() else {
                continue;
            };
            board.set(exit, Tile::Exit);
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
                board.set(*candidate, Tile::Spike);
                let check = solver::analyze(&board, start);
                if check
                    .solution
                    .as_ref()
                    .is_some_and(|p| difficulty.slide_range().contains(&p.len()))
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
            let Some(path) = verified.solution else {
                break;
            };
            if !difficulty.slide_range().contains(&path.len()) {
                break;
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
            return Ok(Stage {
                board,
                start,
                seed,
                difficulty,
                par: path.len(),
            });
        }
    }
    Err(format!(
        "Could not verify a {difficulty} stage for seed {seed}. Retry or choose another seed."
    ))
}

fn assemble(difficulty: Difficulty, rng: &mut impl Rng) -> Board {
    use pieces::Family;
    let (columns, rows, room_w, room_h) = match difficulty {
        Difficulty::Easy => (2, 2, 7, 5),
        Difficulty::Normal => (3, 2, 7, 5),
        Difficulty::Hard => (3, 2, 9, 7),
    };
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
    fn exhausted_budget_returns_replayable_error() {
        let error = generate_with_budget(42, Difficulty::Hard, 0).unwrap_err();
        assert!(error.contains("42") && error.contains("hard"));
    }
}
