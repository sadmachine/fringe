use crate::model::{Board, Direction, Pos, Step, Tile};
use std::collections::{HashMap, HashSet, VecDeque};

pub(super) struct Analysis {
    pub distances: Vec<Option<usize>>,
    pub solution: Option<Vec<Direction>>,
    pub safe_cells: HashSet<Pos>,
    slides: Vec<Slide>,
}

struct Slide {
    from: usize,
    to: Option<usize>,
    cells: Vec<Pos>,
    direction: Direction,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum CollectionError {
    Impossible,
    BudgetExceeded,
}

pub(super) fn full_clear(
    board: &Board,
    start: Pos,
    analysis: &Analysis,
    budget: usize,
) -> Result<Vec<Direction>, CollectionError> {
    if budget == 0 {
        return Err(CollectionError::BudgetExceeded);
    }
    if analysis.solution.is_none() {
        return Err(CollectionError::Impossible);
    }
    let mut signatures = vec![Vec::new(); board.tiles.len()];
    for (edge, slide) in analysis.slides.iter().enumerate() {
        for pos in &slide.cells {
            let index = pos.y * board.width + pos.x;
            if matches!(board.tiles[index], Tile::Dot | Tile::Coin) {
                signatures[index].push(edge);
            }
        }
    }
    // Rewards traversed by precisely the same slides are collected together.
    let mut groups = HashMap::new();
    let mut reward_groups = vec![None; board.tiles.len()];
    for (index, tile) in board.tiles.iter().enumerate() {
        if !matches!(tile, Tile::Dot | Tile::Coin) {
            continue;
        }
        let signature = std::mem::take(&mut signatures[index]);
        if signature.is_empty() {
            return Err(CollectionError::Impossible);
        }
        let next = groups.len();
        reward_groups[index] = Some(*groups.entry(signature).or_insert(next));
    }
    let words = groups.len().div_ceil(64);
    if words == 0 {
        return Ok(analysis.solution.clone().unwrap());
    }
    let mut target = vec![u64::MAX; words];
    if groups.len() % 64 != 0 {
        target[words - 1] = (1u64 << (groups.len() % 64)) - 1;
    }
    struct Move {
        to: Option<usize>,
        rewards: Vec<u64>,
        direction: Direction,
    }
    let mut moves: Vec<Vec<Move>> = (0..board.tiles.len()).map(|_| Vec::new()).collect();
    let mut before_exit = vec![0; words];
    for slide in &analysis.slides {
        let mut rewards = vec![0u64; words];
        for pos in &slide.cells {
            if let Some(group) = reward_groups[pos.y * board.width + pos.x] {
                rewards[group / 64] |= 1 << (group % 64);
            }
        }
        if slide.to.is_some() {
            for (all, reward) in before_exit.iter_mut().zip(&rewards) {
                *all |= reward;
            }
        }
        moves[slide.from].push(Move {
            to: slide.to,
            rewards,
            direction: slide.direction,
        });
    }
    // Anything collectable only on the final slide must fit on one final approach.
    let final_rewards: Vec<_> = target
        .iter()
        .zip(before_exit)
        .map(|(all, before)| all & !before)
        .collect();
    if !moves
        .iter()
        .flatten()
        .any(|step| step.to.is_none() && contains(&step.rewards, &final_rewards))
    {
        return Err(CollectionError::Impossible);
    }
    struct Node {
        at: usize,
        rewards: Vec<u64>,
        previous: Option<(usize, Direction)>,
    }
    let start_index = start.y * board.width + start.x;
    let mut nodes = vec![Node {
        at: start_index,
        rewards: vec![0; words],
        previous: None,
    }];
    let mut visited = vec![Vec::new(); board.tiles.len()];
    visited[start_index].push(0);
    let mut cursor = 0;
    while cursor < nodes.len() {
        for step in &moves[nodes[cursor].at] {
            let rewards: Vec<_> = nodes[cursor]
                .rewards
                .iter()
                .zip(&step.rewards)
                .map(|(have, found)| have | found)
                .collect();
            let Some(to) = step.to else {
                if rewards == target {
                    let mut path = vec![step.direction];
                    let mut previous = cursor;
                    while let Some((parent, direction)) = nodes[previous].previous {
                        path.push(direction);
                        previous = parent;
                    }
                    path.reverse();
                    return Ok(path);
                }
                continue;
            };
            // BFS visits earlier states in no more slides. A prior superset dominates this state.
            if visited[to]
                .iter()
                .any(|prior| contains(&nodes[*prior].rewards, &rewards))
            {
                continue;
            }
            if nodes.len() == budget {
                return Err(CollectionError::BudgetExceeded);
            }
            // Future candidates cost at least this much. Keep only maximal masks in the index;
            // older queued nodes still expand, preserving any shorter routes through them.
            visited[to].retain(|prior| !contains(&rewards, &nodes[*prior].rewards));
            visited[to].push(nodes.len());
            nodes.push(Node {
                at: to,
                rewards,
                previous: Some((cursor, step.direction)),
            });
        }
        cursor += 1;
    }
    Err(CollectionError::Impossible)
}

fn contains(have: &[u64], required: &[u64]) -> bool {
    have.iter()
        .zip(required)
        .all(|(have, required)| have & required == *required)
}

pub(super) fn analyze(board: &Board, start: Pos) -> Analysis {
    let index = |p: Pos| p.y * board.width + p.x;
    let count = board.tiles.len();
    let mut distances = vec![None; count];
    let mut resting_distance = vec![None; count];
    let mut parents: Vec<Option<(Pos, Direction)>> = vec![None; count];
    let mut queue = VecDeque::from([start]);
    distances[index(start)] = Some(0);
    resting_distance[index(start)] = Some(0);
    // None is the terminal exit; lethal and stationary moves produce no edge.
    let mut edges = Vec::new();
    let mut solution = None;
    while let Some(from) = queue.pop_front() {
        let depth = resting_distance[index(from)].unwrap() + 1;
        for direction in Direction::ALL {
            let mut pos = from;
            let mut cells = Vec::new();
            let mut won = false;
            let mut died = false;
            loop {
                match board.step(pos, direction) {
                    Step::Blocked => break,
                    Step::Died(_) => {
                        died = true;
                        break;
                    }
                    Step::Cleared(next) => {
                        cells.push(next);
                        won = true;
                        break;
                    }
                    Step::Moved(next) => {
                        pos = next;
                        cells.push(next);
                    }
                }
            }
            if died || cells.is_empty() {
                continue;
            }
            for cell in &cells {
                let distance = &mut distances[index(*cell)];
                if distance.is_none() || distance.is_some_and(|d| depth < d) {
                    *distance = Some(depth);
                }
            }
            let end = if won { None } else { Some(index(pos)) };
            edges.push(Slide {
                from: index(from),
                to: end,
                cells,
                direction,
            });
            if won {
                if solution.is_none() {
                    let mut path = vec![direction];
                    let mut cursor = from;
                    while let Some((previous, direction)) = parents[index(cursor)] {
                        path.push(direction);
                        cursor = previous;
                    }
                    path.reverse();
                    solution = Some(path);
                }
            } else if resting_distance[index(pos)].is_none() {
                resting_distance[index(pos)] = Some(depth);
                parents[index(pos)] = Some((from, direction));
                queue.push_back(pos);
            }
        }
    }
    let mut reverse = vec![Vec::new(); count];
    let mut can_win = vec![false; count];
    let mut queue = VecDeque::new();
    for edge in &edges {
        if let Some(to) = edge.to {
            reverse[to].push(edge.from);
        } else if !can_win[edge.from] {
            can_win[edge.from] = true;
            queue.push_back(edge.from);
        }
    }
    while let Some(to) = queue.pop_front() {
        for from in &reverse[to] {
            if !can_win[*from] {
                can_win[*from] = true;
                queue.push_back(*from);
            }
        }
    }
    let mut safe_cells = HashSet::new();
    if can_win[index(start)] {
        safe_cells.insert(start);
    }
    for edge in &edges {
        if edge.to.is_none() || edge.to.is_some_and(|to| can_win[to]) {
            safe_cells.extend(
                edge.cells
                    .iter()
                    .copied()
                    .filter(|p| matches!(board.get(*p), Tile::Floor | Tile::Dot | Tile::Coin)),
            );
        }
    }
    Analysis {
        distances,
        solution,
        safe_cells,
        slides: edges,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Tile;

    fn board(rows: &[&str]) -> Board {
        let mut b = Board::filled(rows[0].len(), rows.len(), Tile::Wall);
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                b.set(
                    Pos { x, y },
                    match c {
                        '#' => Tile::Wall,
                        '^' => Tile::Spike,
                        'E' => Tile::Exit,
                        '.' => Tile::Dot,
                        '$' => Tile::Coin,
                        _ => Tile::Floor,
                    },
                );
            }
        }
        b
    }

    fn collection(rows: &[&str], start: Pos) -> Result<Vec<Direction>, CollectionError> {
        let board = board(rows);
        full_clear(&board, start, &analyze(&board, start), 50_000)
    }

    #[test]
    fn full_clear_collects_dots_and_coins_before_entering_exit() {
        let rows = ["#######", "# E $ #", "# ### #", "#  .  #", "#######"];
        let board = board(&rows);
        let start = Pos { x: 1, y: 1 };
        assert_eq!(analyze(&board, start).solution.unwrap().len(), 1);
        let path = full_clear(&board, start, &analyze(&board, start), 50_000).unwrap();
        assert_eq!(
            path,
            vec![
                Direction::Down,
                Direction::Right,
                Direction::Up,
                Direction::Left
            ]
        );
        let mut game = crate::model::Game::new(board, start);
        for direction in path {
            game.launch(direction);
            while game.direction.is_some() {
                game.tick();
            }
        }
        assert_eq!(game.outcome, crate::model::Outcome::Cleared);
        assert_eq!(game.score, 11);
        assert!(!game
            .board
            .tiles
            .iter()
            .any(|tile| matches!(tile, Tile::Dot | Tile::Coin)));
    }

    #[test]
    fn individually_reachable_rewards_can_be_impossible_to_collect_together() {
        let rows = ["#######", "# .E. #", "# ### #", "#     #", "#######"];
        let board = board(&rows);
        let start = Pos { x: 1, y: 1 };
        let analysis = analyze(&board, start);
        assert!(analysis.safe_cells.contains(&Pos { x: 2, y: 1 }));
        assert!(analysis.safe_cells.contains(&Pos { x: 4, y: 1 }));
        assert_eq!(
            full_clear(&board, start, &analysis, 50_000),
            Err(CollectionError::Impossible)
        );
    }

    #[test]
    fn full_clear_can_revisit_a_junction_with_more_rewards() {
        let path = collection(
            &["#######", "###$###", "### ###", "#.   E#", "#######"],
            Pos { x: 3, y: 3 },
        )
        .unwrap();
        assert_eq!(
            path,
            vec![
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right
            ]
        );
    }

    #[test]
    fn empty_collection_has_the_same_par_as_any_percent() {
        let rows = ["#####", "#  E#", "#####"];
        assert_eq!(
            collection(&rows, Pos { x: 1, y: 1 }),
            Ok(vec![Direction::Right])
        );
    }

    #[test]
    fn collection_search_never_crosses_spikes_or_invents_a_par_at_its_budget() {
        let board = board(&["#######", "# . E #", "#^### #", "#  $  #", "#######"]);
        let start = Pos { x: 1, y: 1 };
        let analysis = analyze(&board, start);
        assert_eq!(
            full_clear(&board, start, &analysis, 50_000),
            Err(CollectionError::Impossible)
        );
        assert_eq!(
            full_clear(&board, start, &analysis, 0),
            Err(CollectionError::BudgetExceeded)
        );
        let board = super::tests::board(&["#######", "# E $ #", "# ### #", "#  .  #", "#######"]);
        assert_eq!(
            full_clear(&board, start, &analyze(&board, start), 1),
            Err(CollectionError::BudgetExceeded)
        );
    }

    #[test]
    fn exit_is_reached_in_transit_and_spike_terminates_path() {
        let b = board(&["#######", "#..E.^#", "#######"]);
        let result = analyze(&b, Pos { x: 1, y: 1 });
        assert_eq!(result.solution, Some(vec![Direction::Right]));
        assert!(result.safe_cells.contains(&Pos { x: 2, y: 1 }));
        assert!(!result.safe_cells.contains(&Pos { x: 4, y: 1 }));
        let b = board(&["#######", "#.^.E.#", "#######"]);
        let result = analyze(&b, Pos { x: 1, y: 1 });
        assert!(result.solution.is_none());
        assert!(result.safe_cells.is_empty());
    }

    #[test]
    fn an_open_turn_does_not_allow_stopping_mid_slide() {
        let b = board(&["#######", "#.....#", "###.###", "###E###", "#######"]);
        assert!(analyze(&b, Pos { x: 1, y: 1 }).solution.is_none());
    }

    #[test]
    fn safe_cells_exclude_a_one_way_dead_end() {
        let b = board(&[
            "#########",
            "#..E....#",
            "###.#####",
            "#.......#",
            "#########",
        ]);
        let result = analyze(&b, Pos { x: 3, y: 3 });
        assert!(result.solution.is_some());
        assert!(result.safe_cells.contains(&Pos { x: 3, y: 2 }));
        assert!(!result.safe_cells.contains(&Pos { x: 1, y: 3 }));
        assert!(!result.safe_cells.contains(&Pos { x: 3, y: 1 })); // Exit itself is not a reward.
    }
}
