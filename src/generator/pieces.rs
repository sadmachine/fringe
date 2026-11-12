use crate::model::{Board, Direction, Pos, Step, Tile};
use rand::Rng;

#[derive(Clone, Copy, Debug)]
pub(super) enum Family {
    Switchback,
    Loop,
    Chamber,
}

pub(super) fn make(width: usize, height: usize, family: Family, rng: &mut impl Rng) -> Board {
    let transpose = rng.gen_bool(0.5);
    let (w, h) = if transpose {
        (height, width)
    } else {
        (width, height)
    };
    let mut base = Board::filled(w, h, Tile::Wall);
    match family {
        Family::Switchback => {
            let first_right = rng.gen_bool(0.5);
            let lanes = h.div_ceil(2);
            let turns: Vec<_> = (0..lanes - 1)
                .map(|lane| {
                    if (lane % 2 == 0) == first_right {
                        rng.gen_range(w - 2..w)
                    } else {
                        rng.gen_range(0..2)
                    }
                })
                .collect();
            for lane in 0..lanes {
                let mut left = rng.gen_range(0..2);
                let mut right = rng.gen_range(w - 2..w);
                // Both passages end at their shared bend, so the player can stop and turn.
                for &turn in [
                    lane.checked_sub(1).and_then(|i| turns.get(i)),
                    turns.get(lane),
                ]
                .into_iter()
                .flatten()
                {
                    if turn < w / 2 {
                        left = turn;
                    } else {
                        right = turn;
                    }
                }
                let y = lane * 2;
                for x in left..=right {
                    base.set(Pos { x, y }, Tile::Floor);
                }
                if let Some(&x) = turns.get(lane) {
                    base.set(Pos { x, y: y + 1 }, Tile::Floor);
                }
            }
        }
        Family::Loop => {
            let left = rng.gen_range(0..=1);
            let right = w - 1 - rng.gen_range(0..=1);
            let top = rng.gen_range(0..=1);
            let bottom = h - 1 - rng.gen_range(0..=1);
            for y in top..=bottom {
                for x in left..=right {
                    if x == left || x == right || y == top || y == bottom {
                        base.set(Pos { x, y }, Tile::Floor);
                    }
                }
            }
        }
        Family::Chamber => {
            base.tiles.fill(Tile::Floor);
            // Staggered edge pillars provide stopping points to enter the interior lanes.
            let horizontal_phase = rng.gen_range(0..2);
            let vertical_phase = rng.gen_range(0..2);
            for x in (1..w - 1).step_by(2) {
                let y = if x / 2 % 2 == horizontal_phase {
                    0
                } else {
                    h - 1
                };
                base.set(Pos { x, y }, Tile::Wall);
            }
            for y in (1..h - 1).step_by(2) {
                let x = if y / 2 % 2 == vertical_phase {
                    0
                } else {
                    w - 1
                };
                base.set(Pos { x, y }, Tile::Wall);
                for x in (1..w - 1).step_by(2) {
                    if rng.gen_bool(0.85) {
                        base.set(Pos { x, y }, Tile::Wall);
                    }
                }
            }
            // Opposing edge pillars can enclose a single corner cell; keep it solid.
            for pos in [
                Pos { x: 0, y: 0 },
                Pos { x: w - 1, y: 0 },
                Pos { x: 0, y: h - 1 },
                Pos { x: w - 1, y: h - 1 },
            ] {
                if Direction::ALL
                    .into_iter()
                    .all(|direction| base.step(pos, direction) == Step::Blocked)
                {
                    base.set(pos, Tile::Wall);
                }
            }
        }
    }
    let flip_x = rng.gen_bool(0.5);
    let flip_y = rng.gen_bool(0.5);
    let mut result = Board::filled(width, height, Tile::Wall);
    for y in 0..height {
        for x in 0..width {
            let rx = if flip_x { width - 1 - x } else { x };
            let ry = if flip_y { height - 1 - y } else { y };
            let source = if transpose {
                Pos { x: ry, y: rx }
            } else {
                Pos { x: rx, y: ry }
            };
            result.set(Pos { x, y }, base.get(source));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Pos, Tile};
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use std::collections::{HashSet, VecDeque};

    #[test]
    fn switchback_turns_are_reachable_from_wall_stops() {
        for seed in 0..20 {
            let board = make(
                9,
                7,
                Family::Switchback,
                &mut ChaCha8Rng::seed_from_u64(seed),
            );
            let floors: Vec<_> = (0..7)
                .flat_map(|y| (0..9).map(move |x| Pos { x, y }))
                .filter(|p| board.get(*p) == Tile::Floor)
                .collect();
            let reached = super::super::solver::analyze(&board, floors[0]);
            assert_eq!(
                reached.distances.iter().filter(|d| d.is_some()).count(),
                floors.len(),
                "seed {seed}: a bend needs a wall to stop the slide"
            );
        }
    }

    #[test]
    fn chamber_pillars_provide_access_to_the_interior() {
        for seed in 0..20 {
            let board = make(9, 7, Family::Chamber, &mut ChaCha8Rng::seed_from_u64(seed));
            let floors: Vec<_> = (0..7)
                .flat_map(|y| (0..9).map(move |x| Pos { x, y }))
                .filter(|p| board.get(*p) == Tile::Floor)
                .collect();
            let reached = super::super::solver::analyze(&board, floors[0]);
            assert!(
                reached.distances.iter().filter(|d| d.is_some()).count() * 4 >= floors.len() * 3,
                "seed {seed}: open space inside a chamber must be playable"
            );
        }
    }

    #[test]
    fn every_family_varies_and_remains_physically_connected() {
        for family in [Family::Switchback, Family::Loop, Family::Chamber] {
            let mut layouts = HashSet::new();
            for seed in 0..20 {
                let board = make(9, 7, family, &mut ChaCha8Rng::seed_from_u64(seed));
                layouts.insert(format!("{:?}", board.tiles));
                let floors: HashSet<_> = (0..7)
                    .flat_map(|y| (0..9).map(move |x| Pos { x, y }))
                    .filter(|p| board.get(*p) == Tile::Floor)
                    .collect();
                assert!(floors.len() >= 12 && floors.len() < 63, "{family:?}");
                let start = *floors.iter().next().unwrap();
                let mut visited = HashSet::from([start]);
                let mut queue = VecDeque::from([start]);
                while let Some(p) = queue.pop_front() {
                    for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                        let (Some(x), Some(y)) =
                            (p.x.checked_add_signed(dx), p.y.checked_add_signed(dy))
                        else {
                            continue;
                        };
                        let next = Pos { x, y };
                        if floors.contains(&next) && visited.insert(next) {
                            queue.push_back(next);
                        }
                    }
                }
                assert_eq!(visited, floors, "{family:?} seed {seed}");
            }
            assert!(
                layouts.len() >= 10,
                "{family:?} should have varied geometry"
            );
        }
    }
}
