use fringe::generator::{generate, Difficulty};
use fringe::model::{Board, Direction, Game, Outcome, Pos, Tile};
use std::collections::{HashMap, HashSet, VecDeque};

// Independent cell walker: deliberately does not call Board::step or the generator's solver.
fn slide(board: &Board, from: Pos, direction: Direction) -> Option<(Pos, Vec<Pos>, bool)> {
    let (dx, dy) = match direction {
        Direction::Up => (0, -1),
        Direction::Right => (1, 0),
        Direction::Down => (0, 1),
        Direction::Left => (-1, 0),
    };
    let mut pos = from;
    let mut cells = Vec::new();
    loop {
        let x = pos.x as isize + dx;
        let y = pos.y as isize + dy;
        if x < 0 || y < 0 || x >= board.width as isize || y >= board.height as isize {
            break;
        }
        let next = Pos {
            x: x as usize,
            y: y as usize,
        };
        match board.get(next) {
            Tile::Wall => break,
            Tile::Spike => return None,
            Tile::Exit => {
                cells.push(next);
                return Some((next, cells, true));
            }
            _ => {
                pos = next;
                cells.push(pos);
            }
        }
    }
    if cells.is_empty() {
        None
    } else {
        Some((pos, cells, false))
    }
}

fn verify(board: &Board, start: Pos) -> (Vec<Direction>, HashSet<Pos>) {
    let mut paths = HashMap::from([(start, Vec::new())]);
    let mut queue = VecDeque::from([start]);
    let mut edges = Vec::new();
    let mut solution = None;
    let mut can_win = HashSet::new();
    while let Some(pos) = queue.pop_front() {
        for direction in Direction::ALL {
            if let Some((end, cells, won)) = slide(board, pos, direction) {
                if won {
                    can_win.insert(pos);
                    if solution.is_none() {
                        let mut path = paths[&pos].clone();
                        path.push(direction);
                        solution = Some(path);
                    }
                } else if !paths.contains_key(&end) {
                    let mut path = paths[&pos].clone();
                    path.push(direction);
                    paths.insert(end, path);
                    queue.push_back(end);
                }
                edges.push((pos, end, cells, won));
            }
        }
    }
    loop {
        let before = can_win.len();
        for (from, to, _, won) in &edges {
            if *won || can_win.contains(to) {
                can_win.insert(*from);
            }
        }
        if can_win.len() == before {
            break;
        }
    }
    let mut safe = HashSet::new();
    for (_, to, cells, won) in edges {
        if won || can_win.contains(&to) {
            safe.extend(cells);
        }
    }
    (
        solution.expect("generated exit must be reachable under slide rules"),
        safe,
    )
}

#[test]
fn seeded_stages_are_solvable_reproducible_and_fit_the_terminal() {
    for difficulty in Difficulty::ALL {
        for seed in 0..100 {
            let stage = generate(seed, difficulty).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(
                stage,
                generate(seed, difficulty).unwrap(),
                "replay {difficulty} {seed}"
            );
            let board = &stage.board;
            assert!(board.width * 2 + 4 <= 80 && board.height + 7 <= 24);
            assert_eq!(board.get(stage.start), Tile::Floor);
            let mut exits = 0;
            let mut rewards = 0;
            let (solution, safe) = verify(board, stage.start);
            assert!(
                difficulty.slide_range().contains(&solution.len()),
                "{difficulty} seed {seed} length {}",
                solution.len()
            );
            assert_eq!(solution.len(), stage.par);
            for y in 0..board.height {
                for x in 0..board.width {
                    let pos = Pos { x, y };
                    let tile = board.get(pos);
                    if x == 0 || y == 0 || x == board.width - 1 || y == board.height - 1 {
                        assert_eq!(tile, Tile::Wall);
                    }
                    if tile == Tile::Exit {
                        exits += 1;
                    }
                    if matches!(tile, Tile::Dot | Tile::Coin) {
                        assert!(safe.contains(&pos), "unsafe reward {pos:?}");
                        rewards += 1;
                    }
                }
            }
            assert_eq!(exits, 1);
            assert!(rewards > 0);
            let mut game = Game::new(board.clone(), stage.start);
            for direction in solution {
                game.launch(direction);
                for _ in 0..board.width.max(board.height) {
                    if game.direction.is_none() {
                        break;
                    }
                    game.tick();
                }
            }
            assert_eq!(game.outcome, Outcome::Cleared);
        }
    }
}

#[test]
fn stages_use_the_assembled_map_and_separate_start_from_exit() {
    for difficulty in Difficulty::ALL {
        for seed in 0..100 {
            let stage = generate(seed, difficulty).unwrap_or_else(|e| panic!("{e}"));
            let (_, safe) = verify(&stage.board, stage.start);
            let mut exit = None;
            let mut unusable = Vec::new();
            for y in 0..stage.board.height {
                for x in 0..stage.board.width {
                    let pos = Pos { x, y };
                    match stage.board.get(pos) {
                        Tile::Exit => exit = Some(pos),
                        Tile::Floor | Tile::Dot | Tile::Coin
                            if pos != stage.start && !safe.contains(&pos) =>
                        {
                            unusable.push(pos)
                        }
                        _ => {}
                    }
                }
            }
            assert!(unusable.is_empty(), "{difficulty} seed {seed}: {} floor cells are outside safe winning routes, including {:?}", unusable.len(), &unusable[..unusable.len().min(5)]);
            let exit = exit.unwrap();
            assert!(stage.start.x.abs_diff(exit.x) * 2 >= stage.board.width - 2
                && stage.start.y.abs_diff(exit.y) * 2 >= stage.board.height - 2,
                "{difficulty} seed {seed}: start {:?} and exit {exit:?} occupy one corner of a {}x{} map", stage.start, stage.board.width, stage.board.height);
        }
    }
}
