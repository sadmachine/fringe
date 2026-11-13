use crate::model::{Board, Direction, Pos, Tile};
use rand::{seq::SliceRandom, Rng};

#[derive(Clone, Copy, Debug)]
pub(super) enum Family {
    Switchback,
    IslandReturn,
    LoopChain,
    RewardPocket,
    TouchRun,
    MixedCrossing,
}

impl Family {
    #[cfg(test)]
    const ALL: [Self; 6] = [
        Self::Switchback,
        Self::IslandReturn,
        Self::LoopChain,
        Self::RewardPocket,
        Self::TouchRun,
        Self::MixedCrossing,
    ];

    fn sides(self) -> &'static [Direction] {
        use Direction::*;
        match self {
            Self::Switchback | Self::TouchRun => &[Up, Right],
            Self::RewardPocket => &[Up, Right, Down],
            _ => &Direction::ALL,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Port {
    pub pos: Pos,
    pub direction: Direction,
}

#[derive(Clone, Debug)]
pub(super) struct Piece {
    pub board: Board,
    pub ports: Vec<Port>,
}

#[derive(Clone, Copy, Debug)]
struct Transform {
    transpose: bool,
    flip_x: bool,
    flip_y: bool,
}

impl Transform {
    const ALL: [Self; 8] = [
        Self {
            transpose: false,
            flip_x: false,
            flip_y: false,
        },
        Self {
            transpose: false,
            flip_x: true,
            flip_y: false,
        },
        Self {
            transpose: false,
            flip_x: false,
            flip_y: true,
        },
        Self {
            transpose: false,
            flip_x: true,
            flip_y: true,
        },
        Self {
            transpose: true,
            flip_x: false,
            flip_y: false,
        },
        Self {
            transpose: true,
            flip_x: true,
            flip_y: false,
        },
        Self {
            transpose: true,
            flip_x: false,
            flip_y: true,
        },
        Self {
            transpose: true,
            flip_x: true,
            flip_y: true,
        },
    ];

    fn direction(self, direction: Direction) -> Direction {
        use Direction::*;
        let direction = if self.transpose {
            match direction {
                Up => Left,
                Right => Down,
                Down => Right,
                Left => Up,
            }
        } else {
            direction
        };
        let direction = if self.flip_x {
            match direction {
                Left => Right,
                Right => Left,
                other => other,
            }
        } else {
            direction
        };
        if self.flip_y {
            match direction {
                Up => Down,
                Down => Up,
                other => other,
            }
        } else {
            direction
        }
    }

    fn apply(self, piece: Piece) -> Piece {
        let source = &piece.board;
        let (width, height) = if self.transpose {
            (source.height, source.width)
        } else {
            (source.width, source.height)
        };
        let map = |pos: Pos| {
            let (x, y) = if self.transpose {
                (pos.y, pos.x)
            } else {
                (pos.x, pos.y)
            };
            Pos {
                x: if self.flip_x { width - 1 - x } else { x },
                y: if self.flip_y { height - 1 - y } else { y },
            }
        };
        let mut board = Board::filled(width, height, Tile::Wall);
        for y in 0..source.height {
            for x in 0..source.width {
                let pos = Pos { x, y };
                board.set(map(pos), source.get(pos));
            }
        }
        Piece {
            board,
            ports: piece
                .ports
                .into_iter()
                .map(|port| Port {
                    pos: map(port.pos),
                    direction: self.direction(port.direction),
                })
                .collect(),
        }
    }
}

pub(super) fn make(
    width: usize,
    height: usize,
    family: Family,
    required: &[Direction],
    rng: &mut impl Rng,
) -> Piece {
    let mut transforms = Transform::ALL;
    transforms.shuffle(rng);
    let transform = transforms
        .into_iter()
        .find(|transform| {
            required.iter().all(|side| {
                family
                    .sides()
                    .iter()
                    .any(|d| transform.direction(*d) == *side)
            })
        })
        .expect("templates support the adjacent sides required by the room grid");
    let (w, h) = if transform.transpose {
        (height, width)
    } else {
        (width, height)
    };
    transform.apply(build(w, h, family, rng))
}

fn line(board: &mut Board, from: Pos, to: Pos) {
    debug_assert!(from.x == to.x || from.y == to.y);
    for y in from.y.min(to.y)..=from.y.max(to.y) {
        for x in from.x.min(to.x)..=from.x.max(to.x) {
            board.set(Pos { x, y }, Tile::Floor);
        }
    }
}

fn outline(board: &mut Board, left: usize, top: usize, right: usize, bottom: usize) {
    let corners = [
        Pos { x: left, y: top },
        Pos { x: right, y: top },
        Pos {
            x: right,
            y: bottom,
        },
        Pos { x: left, y: bottom },
    ];
    for i in 0..4 {
        line(board, corners[i], corners[(i + 1) % 4]);
    }
}

fn ring(
    width: usize,
    height: usize,
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
) -> Piece {
    let mut board = Board::filled(width, height, Tile::Wall);
    outline(&mut board, left, top, right, bottom);
    // Each corner keeps one stopping wall. Opening both faces would erase the turn.
    let ports = vec![
        Port {
            pos: Pos { x: 0, y: top },
            direction: Direction::Left,
        },
        Port {
            pos: Pos { x: right, y: 0 },
            direction: Direction::Up,
        },
        Port {
            pos: Pos {
                x: width - 1,
                y: bottom,
            },
            direction: Direction::Right,
        },
        Port {
            pos: Pos {
                x: left,
                y: height - 1,
            },
            direction: Direction::Down,
        },
    ];
    for (port, corner) in ports.iter().zip([
        Pos { x: left, y: top },
        Pos { x: right, y: top },
        Pos {
            x: right,
            y: bottom,
        },
        Pos { x: left, y: bottom },
    ]) {
        line(&mut board, port.pos, corner);
    }
    Piece { board, ports }
}

fn switchback(width: usize, height: usize, rng: &mut impl Rng) -> Piece {
    let mut board = Board::filled(width, height, Tile::Wall);
    let left = if width > 7 { rng.gen_range(1..=2) } else { 1 };
    let right = if width > 7 {
        rng.gen_range(width - 3..=width - 2)
    } else {
        width - 2
    };
    let middle = rng.gen_range(3..=height - 4);
    let top_start = rng.gen_range(1..=left);
    let bottom_end = rng.gen_range(right..=width - 2);
    for (y, start, end) in [
        (1, top_start, right),
        (middle, left, right),
        (height - 2, left, bottom_end),
    ] {
        line(&mut board, Pos { x: start, y }, Pos { x: end, y });
    }
    line(
        &mut board,
        Pos { x: right, y: 1 },
        Pos {
            x: right,
            y: middle,
        },
    );
    line(
        &mut board,
        Pos { x: left, y: middle },
        Pos {
            x: left,
            y: height - 2,
        },
    );
    let ports = vec![
        Port {
            pos: Pos { x: top_start, y: 0 },
            direction: Direction::Up,
        },
        Port {
            pos: Pos {
                x: width - 1,
                y: height - 2,
            },
            direction: Direction::Right,
        },
    ];
    line(&mut board, ports[0].pos, Pos { x: top_start, y: 1 });
    line(
        &mut board,
        Pos {
            x: bottom_end,
            y: height - 2,
        },
        ports[1].pos,
    );
    Piece { board, ports }
}

fn loop_chain(width: usize, height: usize, rng: &mut impl Rng) -> Piece {
    let transpose = height > width;
    let (w, h) = if transpose {
        (height, width)
    } else {
        (width, height)
    };
    let divider = rng.gen_range(4..=w - 5);
    let mut board = Board::filled(w, h, Tile::Wall);
    outline(&mut board, 1, 1, divider - 1, h - 2);
    outline(&mut board, divider + 1, 1, w - 2, h - 2);
    board.set(Pos { x: divider, y: 1 }, Tile::Floor);
    // Leave both ends of the shared top lane capped. Each loop can then be
    // entered from that lane and returned through its inner vertical leg.
    let ports = vec![
        Port {
            pos: Pos { x: 0, y: h - 2 },
            direction: Direction::Left,
        },
        Port {
            pos: Pos { x: w - 2, y: 0 },
            direction: Direction::Up,
        },
        Port {
            pos: Pos { x: w - 1, y: h - 2 },
            direction: Direction::Right,
        },
        Port {
            pos: Pos {
                x: divider - 1,
                y: h - 1,
            },
            direction: Direction::Down,
        },
    ];
    for (port, corner) in ports.iter().zip([
        Pos { x: 1, y: h - 2 },
        Pos { x: w - 2, y: 1 },
        Pos { x: w - 2, y: h - 2 },
        Pos {
            x: divider - 1,
            y: h - 2,
        },
    ]) {
        line(&mut board, port.pos, corner);
    }
    let piece = Piece { board, ports };
    let piece = if rng.gen_bool(0.5) {
        Transform {
            transpose: false,
            flip_x: false,
            flip_y: true,
        }
        .apply(piece)
    } else {
        piece
    };
    if transpose {
        Transform {
            transpose: true,
            flip_x: false,
            flip_y: false,
        }
        .apply(piece)
    } else {
        piece
    }
}

fn reward_pocket(width: usize, height: usize, rng: &mut impl Rng) -> Piece {
    let left = width / 2;
    let right = width - 2;
    let bottom = height - 2;
    let mut board = Board::filled(width, height, Tile::Wall);
    outline(&mut board, left, 1, right, bottom);
    let reward = Pos {
        x: rng.gen_range(1..left),
        y: 1,
    };
    line(&mut board, reward, Pos { x: left, y: 1 });
    board.set(reward, Tile::Coin);
    let ports = vec![
        Port {
            pos: Pos { x: right, y: 0 },
            direction: Direction::Up,
        },
        Port {
            pos: Pos {
                x: width - 1,
                y: bottom,
            },
            direction: Direction::Right,
        },
        Port {
            pos: Pos {
                x: left,
                y: height - 1,
            },
            direction: Direction::Down,
        },
    ];
    for (port, corner) in ports.iter().zip([
        Pos { x: right, y: 1 },
        Pos {
            x: right,
            y: bottom,
        },
        Pos { x: left, y: bottom },
    ]) {
        line(&mut board, port.pos, corner);
    }
    Piece { board, ports }
}

fn place_run(board: &mut Board, lane: &[Pos], tile: Tile, rng: &mut impl Rng) {
    let length = rng.gen_range(2..=lane.len().min(4));
    let start = rng.gen_range(0..=lane.len() - length);
    for pos in &lane[start..start + length] {
        board.set(*pos, tile);
    }
}

fn build(width: usize, height: usize, family: Family, rng: &mut impl Rng) -> Piece {
    match family {
        Family::Switchback => switchback(width, height, rng),
        Family::IslandReturn => ring(
            width,
            height,
            rng.gen_range(1..=2),
            rng.gen_range(1..=2),
            width - 1 - rng.gen_range(1..=2),
            height - 1 - rng.gen_range(1..=2),
        ),
        Family::LoopChain => loop_chain(width, height, rng),
        Family::RewardPocket => reward_pocket(width, height, rng),
        Family::TouchRun => {
            let mut piece = switchback(width, height, rng);
            let lanes: Vec<Vec<_>> = (1..height - 1)
                .map(|y| {
                    (1..width - 1)
                        .map(|x| Pos { x, y })
                        .filter(|p| {
                            piece.board.get(*p) == Tile::Floor
                                && piece.board.get(Pos { x: p.x, y: p.y - 1 }) == Tile::Wall
                                && piece.board.get(Pos { x: p.x, y: p.y + 1 }) == Tile::Wall
                                && piece.board.get(Pos { x: p.x - 1, y: p.y }) == Tile::Floor
                                && piece.board.get(Pos { x: p.x + 1, y: p.y }) == Tile::Floor
                        })
                        .collect()
                })
                .filter(|lane: &Vec<Pos>| lane.len() >= 2)
                .collect();
            place_run(
                &mut piece.board,
                lanes
                    .choose(rng)
                    .expect("switchback has a straight hazard lane"),
                Tile::TouchTrap,
                rng,
            );
            piece
        }
        Family::MixedCrossing => {
            let horizontal = width > height;
            let (left, right) = if horizontal {
                (1, width - 2)
            } else {
                (rng.gen_range(1..=2), width - 1 - rng.gen_range(1..=2))
            };
            let (top, bottom) = if horizontal {
                (rng.gen_range(1..=2), height - 1 - rng.gen_range(1..=2))
            } else {
                (1, height - 2)
            };
            let mut piece = ring(width, height, left, top, right, bottom);
            let first: Vec<_> = if horizontal {
                (left + 1..right).map(|x| Pos { x, y: top }).collect()
            } else {
                (top + 1..bottom).map(|y| Pos { x: left, y }).collect()
            };
            let second: Vec<_> = if horizontal {
                (left + 1..right).map(|x| Pos { x, y: bottom }).collect()
            } else {
                (top + 1..bottom).map(|y| Pos { x: right, y }).collect()
            };
            place_run(&mut piece.board, &first, Tile::TimedSpike, rng);
            place_run(&mut piece.board, &second, Tile::TouchTrap, rng);
            // Fixed spikes form a face of the internal island, leaving its
            // surrounding travel lanes and corner stops intact.
            for offset in 1..=2 {
                piece.board.set(
                    Pos {
                        x: left + if horizontal { offset } else { 1 },
                        y: top + if horizontal { 1 } else { offset },
                    },
                    Tile::Spike,
                );
            }
            piece
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        generator::solver,
        model::{Game, Outcome, SpikePhase},
    };
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use std::collections::HashSet;

    fn traversable(tile: Tile) -> bool {
        !matches!(tile, Tile::Wall | Tile::Spike)
    }

    #[test]
    fn every_transform_preserves_ports_and_slide_access_to_the_whole_template() {
        for family in Family::ALL {
            for seed in 0..10 {
                let base = build(9, 7, family, &mut ChaCha8Rng::seed_from_u64(seed));
                for transform in Transform::ALL {
                    let piece = transform.apply(base.clone());
                    let board = &piece.board;
                    for y in 0..board.height {
                        for x in 0..board.width {
                            let pos = Pos { x, y };
                            if x == 0 || y == 0 || x + 1 == board.width || y + 1 == board.height {
                                assert_eq!(
                                    board.get(pos) == Tile::Floor,
                                    piece.ports.iter().any(|p| p.pos == pos),
                                    "{family:?} {seed} {transform:?}: boundary {pos:?}"
                                );
                            }
                        }
                    }
                    for port in &piece.ports {
                        assert!(match port.direction {
                            Direction::Up => port.pos.y == 0,
                            Direction::Right => port.pos.x + 1 == board.width,
                            Direction::Down => port.pos.y + 1 == board.height,
                            Direction::Left => port.pos.x == 0,
                        });
                        let analysis = solver::analyze(board, port.pos);
                        for (index, tile) in board.tiles.iter().enumerate() {
                            if traversable(*tile) {
                                assert!(analysis.distances[index].is_some(), "{family:?} {seed} {transform:?}: cell {index} inaccessible from {port:?}");
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn compatible_ports_fit_every_room_corner_and_keep_requested_dimensions() {
        use Direction::*;
        for family in Family::ALL {
            for required in [[Right, Down], [Left, Down], [Up, Right], [Up, Left]] {
                for seed in 0..10 {
                    let piece = make(
                        9,
                        7,
                        family,
                        &required,
                        &mut ChaCha8Rng::seed_from_u64(seed),
                    );
                    assert_eq!((piece.board.width, piece.board.height), (9, 7));
                    assert!(required
                        .iter()
                        .all(|d| piece.ports.iter().any(|p| p.direction == *d)));
                }
            }
        }
    }

    #[test]
    fn rectangular_transpose_and_reflection_move_tiles_and_port_directions_together() {
        let mut board = Board::filled(3, 2, Tile::Wall);
        board.set(Pos { x: 0, y: 1 }, Tile::Coin);
        let piece = Piece {
            board,
            ports: vec![Port {
                pos: Pos { x: 2, y: 0 },
                direction: Direction::Up,
            }],
        };
        let transformed = Transform {
            transpose: true,
            flip_x: true,
            flip_y: false,
        }
        .apply(piece);
        assert_eq!((transformed.board.width, transformed.board.height), (2, 3));
        assert_eq!(transformed.board.get(Pos { x: 0, y: 0 }), Tile::Coin);
        assert_eq!(
            transformed.ports,
            vec![Port {
                pos: Pos { x: 1, y: 2 },
                direction: Direction::Right
            }]
        );
    }

    #[test]
    fn template_variations_change_geometry_or_hazard_runs() {
        for family in Family::ALL {
            let layouts: HashSet<_> = (0..20)
                .map(|seed| {
                    format!(
                        "{:?}",
                        build(11, 7, family, &mut ChaCha8Rng::seed_from_u64(seed))
                            .board
                            .tiles
                    )
                })
                .collect();
            assert!(
                layouts.len() >= 2,
                "{family:?} needs meaningful variations before rotation"
            );
        }
    }

    #[test]
    fn transformed_challenges_have_timed_playable_paths_between_ports() {
        for family in [Family::TouchRun, Family::MixedCrossing] {
            for transform in Transform::ALL {
                let piece =
                    transform.apply(build(9, 7, family, &mut ChaCha8Rng::seed_from_u64(42)));
                for start in &piece.ports {
                    for exit in &piece.ports {
                        if start.pos == exit.pos {
                            continue;
                        }
                        let mut board = piece.board.clone();
                        board.set(exit.pos, Tile::Exit);
                        let analysis = solver::analyze(&board, start.pos);
                        let path = analysis.solution.unwrap();
                        let mut game = Game::new(board, start.pos);
                        for direction in &path {
                            for _ in 0..70 {
                                game.tick();
                            }
                            for _ in 0..100 {
                                let previous = game.spike_phase();
                                game.tick();
                                if previous == SpikePhase::Active
                                    && game.spike_phase() == SpikePhase::Retracted
                                {
                                    break;
                                }
                            }
                            game.launch(*direction);
                            while game.direction.is_some() {
                                game.tick();
                            }
                        }
                        assert_eq!(game.outcome, Outcome::Cleared, "{family:?} {transform:?}");
                        assert_eq!(game.slides as usize, path.len());
                    }
                }
            }
        }
    }
}
