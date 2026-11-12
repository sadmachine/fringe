#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile { Wall, Floor, Dot, Coin, Spike, Exit }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction { Up, Right, Down, Left }

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
pub enum Step { Blocked, Moved(Pos), Died(Pos), Cleared(Pos) }

impl Board {
    pub fn filled(width: usize, height: usize, tile: Tile) -> Self {
        Self { width, height, tiles: vec![tile; width * height] }
    }
    pub fn get(&self, pos: Pos) -> Tile { self.tiles[pos.y * self.width + pos.x] }
    pub fn set(&mut self, pos: Pos, tile: Tile) { self.tiles[pos.y * self.width + pos.x] = tile; }
    pub fn step(&self, pos: Pos, direction: Direction) -> Step {
        let (dx, dy) = match direction {
            Direction::Up => (0, -1), Direction::Right => (1, 0),
            Direction::Down => (0, 1), Direction::Left => (-1, 0),
        };
        let Some(x) = pos.x.checked_add_signed(dx) else { return Step::Blocked; };
        let Some(y) = pos.y.checked_add_signed(dy) else { return Step::Blocked; };
        if x >= self.width || y >= self.height { return Step::Blocked; }
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
pub enum Outcome { Playing, Died, Cleared }

pub struct Game {
    pub board: Board,
    pub player: Pos,
    pub direction: Option<Direction>,
    pub outcome: Outcome,
    pub score: u32,
    pub slides: u32,
}

impl Game {
    pub fn new(board: Board, start: Pos) -> Self {
        Self { board, player: start, direction: None, outcome: Outcome::Playing, score: 0, slides: 0 }
    }
    pub fn launch(&mut self, direction: Direction) {
        if self.outcome == Outcome::Playing && self.direction.is_none()
            && self.board.step(self.player, direction) != Step::Blocked {
            self.direction = Some(direction);
            self.slides += 1;
        }
    }
    pub fn tick(&mut self) {
        let Some(direction) = self.direction else { return; };
        match self.board.step(self.player, direction) {
            Step::Blocked => { self.direction = None; return; }
            Step::Moved(pos) => {
                self.player = pos;
                self.score += match self.board.get(pos) { Tile::Dot => 1, Tile::Coin => 10, _ => 0 };
                self.board.set(pos, Tile::Floor);
            }
            Step::Died(pos) => { self.player = pos; self.outcome = Outcome::Died; }
            Step::Cleared(pos) => { self.player = pos; self.outcome = Outcome::Cleared; }
        }
        if self.outcome != Outcome::Playing || self.board.step(self.player, direction) == Step::Blocked {
            self.direction = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corridor(tiles: &[Tile]) -> Game {
        let mut board = Board::filled(tiles.len() + 2, 3, Tile::Wall);
        for (x, tile) in tiles.iter().enumerate() { board.set(Pos { x: x + 1, y: 1 }, *tile); }
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
        game.tick(); game.tick();
        assert_eq!(game.score, 11);
        game.launch(Direction::Left);
        game.tick(); game.tick();
        assert_eq!(game.score, 11);
        assert_eq!(game.board.get(Pos { x: 2, y: 1 }), Tile::Floor);
    }

    #[test]
    fn spike_kills_in_transit_and_prevents_further_movement() {
        let mut game = corridor(&[Tile::Floor, Tile::Spike, Tile::Coin]);
        game.launch(Direction::Right); game.tick();
        assert_eq!(game.outcome, Outcome::Died);
        assert_eq!(game.player.x, 2);
        game.launch(Direction::Right); game.tick();
        assert_eq!(game.player.x, 2);
        assert_eq!(game.score, 0);
    }

    #[test]
    fn exit_clears_in_transit_without_collecting_every_dot() {
        let mut game = corridor(&[Tile::Floor, Tile::Exit, Tile::Dot]);
        game.launch(Direction::Right); game.tick();
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
}
