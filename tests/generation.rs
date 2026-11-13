use fringe::generator::{generate, Difficulty};
use fringe::model::{Board, Direction, Game, Outcome, Pos, SpikePhase, Tile};
use std::collections::{HashMap, HashSet, VecDeque};

#[test]
fn hazards_form_contiguous_challenge_runs_instead_of_isolated_cells() {
    for difficulty in Difficulty::ALL {
        for seed in 0..10 {
            let stage = generate(seed, difficulty).unwrap();
            for kind in [Tile::TimedSpike, Tile::TouchTrap] {
                let cells: HashSet<_> = (0..stage.board.height)
                    .flat_map(|y| (0..stage.board.width).map(move |x| Pos { x, y }))
                    .filter(|p| stage.board.get(*p) == kind)
                    .collect();
                assert!(
                    cells.len() >= 2,
                    "{difficulty} seed {seed}: {kind:?} needs a run"
                );
                for pos in &cells {
                    assert!(
                        cells
                            .iter()
                            .any(|other| pos.x.abs_diff(other.x) + pos.y.abs_diff(other.y) == 1),
                        "{difficulty} seed {seed}: isolated {kind:?} at {pos:?}"
                    );
                }
            }
        }
    }
}

// Independent cell walker: deliberately does not call Board::step or the generator's solver.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Inventory {
    Uncollected,
    Armed,
    Spent,
}

type State = (Pos, Inventory);

#[test]
fn shield_can_be_collected_before_the_final_exit_slide() {
    let stage = generate(42, Difficulty::Easy).unwrap();
    let start = (stage.start, Inventory::Uncollected);
    let mut queue = VecDeque::from([start]);
    let mut seen = HashSet::from([start]);
    let mut armed = false;
    while let Some(state) = queue.pop_front() {
        for direction in Direction::ALL {
            if let Some((to, _, false)) = slide(&stage.board, state, direction) {
                armed |= to.1 == Inventory::Armed;
                if seen.insert(to) {
                    queue.push_back(to);
                }
            }
        }
    }
    assert!(armed, "pickup should be usable before the stage ends");
}

fn slide(board: &Board, from: State, direction: Direction) -> Option<(State, Vec<Pos>, bool)> {
    let (dx, dy) = match direction {
        Direction::Up => (0, -1),
        Direction::Right => (1, 0),
        Direction::Down => (0, 1),
        Direction::Left => (-1, 0),
    };
    let (mut pos, mut inventory) = from;
    let mut protected = false;
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
            Tile::Exit => {
                cells.push(next);
                return Some(((next, inventory), cells, true));
            }
            _ => {
                if board.get(next) == Tile::Shield && inventory == Inventory::Uncollected {
                    inventory = Inventory::Armed;
                }
                if board.get(next) == Tile::Spike && !protected {
                    if inventory != Inventory::Armed {
                        return None;
                    }
                    inventory = Inventory::Spent;
                    protected = true;
                }
                pos = next;
                cells.push(pos);
            }
        }
    }
    if cells.is_empty() || board.get(pos) == Tile::Spike {
        None
    } else {
        Some(((pos, inventory), cells, false))
    }
}

fn verify(board: &Board, start: Pos) -> (Vec<Direction>, HashSet<Pos>) {
    let start = (start, Inventory::Uncollected);
    let mut paths = HashMap::from([(start, Vec::new())]);
    let mut queue = VecDeque::from([start]);
    let mut edges = Vec::new();
    let mut solution = None;
    let mut can_win = HashSet::new();
    while let Some(pos) = queue.pop_front() {
        for direction in Direction::ALL {
            if let Some((end, cells, won)) = slide(board, pos, direction) {
                assert!(
                    cells.len() < 50,
                    "every slide fits in both hazards' safe windows"
                );
                assert!(
                    !matches!(
                        board.get(end.0),
                        Tile::TimedSpike | Tile::TouchTrap | Tile::Spike
                    ),
                    "waiting must be safe"
                );
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
fn full_clear_minimum_matches_an_independent_search_on_small_stages() {
    for seed in 0..5 {
        let stage = generate(seed, Difficulty::Easy).unwrap();
        let rewards: HashMap<_, _> = (0..stage.board.height)
            .flat_map(|y| (0..stage.board.width).map(move |x| Pos { x, y }))
            .filter(|p| matches!(stage.board.get(*p), Tile::Dot | Tile::Coin))
            .enumerate()
            .map(|(bit, pos)| (pos, bit))
            .collect();
        assert!(
            rewards.len() < 64,
            "oracle fixture must fit its simple cell mask"
        );
        let target = (1u64 << rewards.len()) - 1;
        let start = (stage.start, Inventory::Uncollected);
        let mut queue = VecDeque::from([(start, 0u64, 0)]);
        let mut visited = HashSet::from([(start, 0u64)]);
        let mut minimum = None;
        while let Some((pos, collected, slides)) = queue.pop_front() {
            for direction in Direction::ALL {
                if let Some((end, cells, won)) = slide(&stage.board, pos, direction) {
                    let mut found = collected;
                    for cell in cells {
                        if let Some(bit) = rewards.get(&cell) {
                            found |= 1u64 << bit;
                        }
                    }
                    if won {
                        if found == target {
                            minimum = Some(slides + 1);
                            break;
                        }
                    } else if visited.insert((end, found)) {
                        queue.push_back((end, found, slides + 1));
                    }
                }
            }
            if minimum.is_some() {
                break;
            }
        }
        assert_eq!(
            minimum,
            Some(stage.pars.full_clear_minimum),
            "easy seed {seed}"
        );
    }
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
            assert_eq!(
                board
                    .tiles
                    .iter()
                    .filter(|tile| **tile == Tile::Shield)
                    .count(),
                1
            );
            let mut exits = 0;
            let mut rewards = 0;
            let timed_spikes = board
                .tiles
                .iter()
                .filter(|t| **t == Tile::TimedSpike)
                .count();
            assert!(
                timed_spikes > 0,
                "{difficulty} seed {seed} has no timed wall"
            );
            assert!(
                board.tiles.contains(&Tile::TouchTrap),
                "{difficulty} seed {seed} has no touch trap"
            );
            let (solution, safe) = verify(board, stage.start);
            assert!(
                difficulty.slide_range().contains(&solution.len()),
                "{difficulty} seed {seed} length {}",
                solution.len()
            );
            assert_eq!(solution.len(), stage.pars.any_percent_minimum);
            assert!(stage.pars.full_clear_minimum >= stage.pars.any_percent_minimum);
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
                    if matches!(tile, Tile::Dot | Tile::Coin | Tile::Shield) {
                        assert!(safe.contains(&pos), "unsafe reward {pos:?}");
                        rewards += 1;
                    }
                    if matches!(tile, Tile::TimedSpike | Tile::TouchTrap) {
                        assert!(safe.contains(&pos), "inaccessible spike wall {pos:?}");
                    }
                }
            }
            assert_eq!(exits, 1);
            assert!(rewards > 0);
            let mut game = Game::new(board.clone(), stage.start);
            // Begin at different cycle offsets, then wait for fresh retraction
            // before each slide. Waiting must preserve the exact slide minimum.
            for _ in 0..seed % 100 {
                game.tick();
            }
            for direction in solution {
                for _ in 0..70 {
                    game.tick();
                }
                for _ in 0..100 {
                    let before = game.spike_phase();
                    game.tick();
                    if before == SpikePhase::Active && game.spike_phase() == SpikePhase::Retracted {
                        break;
                    }
                }
                assert_eq!(game.outcome, Outcome::Playing);
                game.launch(direction);
                for _ in 0..board.width.max(board.height) {
                    if game.direction.is_none() {
                        break;
                    }
                    game.tick();
                }
            }
            assert_eq!(game.outcome, Outcome::Cleared);
            assert_eq!(game.slides as usize, stage.pars.any_percent_minimum);
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
                        Tile::Floor
                        | Tile::Dot
                        | Tile::Coin
                        | Tile::Shield
                        | Tile::TimedSpike
                        | Tile::TouchTrap
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
