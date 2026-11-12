use crate::model::{Board, Direction, Pos, Step, Tile};
use std::collections::{HashSet, VecDeque};

pub(super) struct Analysis {
    pub distances: Vec<Option<usize>>,
    pub solution: Option<Vec<Direction>>,
    pub safe_cells: HashSet<Pos>,
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
    let mut edges: Vec<(usize, Option<usize>, Vec<Pos>)> = Vec::new();
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
            edges.push((index(from), end, cells));
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
    for (from, to, _) in &edges {
        if let Some(to) = to {
            reverse[*to].push(*from);
        } else if !can_win[*from] {
            can_win[*from] = true;
            queue.push_back(*from);
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
    for (_, to, cells) in edges {
        if to.is_none() || to.is_some_and(|to| can_win[to]) {
            safe_cells.extend(
                cells
                    .into_iter()
                    .filter(|p| matches!(board.get(*p), Tile::Floor | Tile::Dot | Tile::Coin)),
            );
        }
    }
    Analysis {
        distances,
        solution,
        safe_cells,
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
                        _ => Tile::Floor,
                    },
                );
            }
        }
        b
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
