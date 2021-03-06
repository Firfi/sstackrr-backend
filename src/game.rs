// the main game file; for better testing and portability, it's immutable

use std::str::SplitWhitespace;
use strum_macros;
use async_graphql::Enum;
use crate::db::GameStateSerialized;
use crate::game::Player::{Blue, Red};

// code assumes our field is at least 1x1
const MIN_DIM: u8 = 1;
pub const WIN_LEN: u8 = 4;

const SERIALIZATION_COL_SEPARATOR: &str = " "; // "separate" cols from each other, but "split" rows
// reuse and keep near SERIALIZATION_COL_SEPARATOR
fn split_row(s: &str) -> SplitWhitespace {
    s.split_whitespace()
}
const SERIALIZATION_ROW_SEPARATOR: &str = "\n";


// Vs. red and yellow for connect-4. Because it's a statement. "We're not connect-4!"
// we assume Red is always going first. like in Chess.
#[derive(Enum, Eq, PartialEq, Debug, Clone, Copy, strum_macros::Display)]
pub enum Player {
    Red,
    Blue
}

const FIRST_PLAYER: Player = Player::Red;

#[derive(Enum, Eq, PartialEq, Debug, Clone, Copy, strum_macros::Display)]
pub enum Side {
    Left,
    Right
}

type Height = u8; // the vertical axis, sides are to the left/right of it

pub type Turn = (Player, Height, Side);
pub type Move = (Height, Side);
pub type CoordsHistory = Vec<Coords>;
type Cell = Option<Player>;
type Field = Vec<Cell>;

#[derive(Clone, Debug)]
pub struct State {
    size_x: u8,
    size_y: u8,
    coords_history: CoordsHistory, // actually, we can do with Only this field
    field: Field, // derivative to History+sizes but here for convenience and performance
    winner_cache: Cell
}

pub(crate) type Coords = (u8, u8);

fn calc_field_index(size_x: u8, x: u8, y: u8) -> u8 {
    y * size_x/*or size_y?*/ + x
}

fn calc_coords(size_x: u8, index: u8) -> Coords {
    (index % size_x, index / size_x)
}

// anything to do directly with the "coords" on the field
pub trait MatrixOperations {
    fn calc_field_index(&self, x: u8, y: u8) -> u8;
    fn get_cell(&self, x: u8, y: u8) -> Result<Cell, String>;
    fn next_cell_towards(&self, direction: Side, y: u8) -> Result<Option<Coords>, String>;
    fn size_x(&self) -> u8;
    fn size_y(&self) -> u8;
    fn line_iterators(&self) -> Vec<fn(u8, u8) -> Vec<Vec<Coords>>>;
    fn lines(&self) -> Vec<Vec<Vec<Coords>>>;
}

impl MatrixOperations for State {
    // matrix coordinate -> array index
    fn calc_field_index(&self, x: u8, y: u8) -> u8 {
        calc_field_index(self.size_x, x, y)
    }
    // get cell at x, y
    fn get_cell(&self, x: u8, y: u8) -> Result<Cell, String> {
        let (size_x, size_y, field) = (self.size_x, self.size_y, &self.field);
        if x >= size_x || y >= size_y {
            return Err(format!("out of bounds {} {}", x, y));
        }
        let cell = self.field[self.calc_field_index(x, y) as usize].clone();
        Ok(cell)
    }
    // where a new piece would land; empty space or nothing
    fn next_cell_towards(&self, direction: Side, y: u8) -> Result<Option<Coords>, String> {
        if y >= self.size_y {
            return Err(format!("out of bounds {}", y));
        }
        let size_x = self.size_x;
        for i in 0..size_x {
            let x = match direction {
                Side::Left => i,
                Side::Right => size_x - i - 1,
            };
            let cell = self.get_cell(x, y)?;
            if cell.is_none() {
                return Ok(Some((x, y)));
            }
        }
        Ok(None)
    }
    fn size_x(&self) -> u8 {
        self.size_x
    }
    fn size_y(&self) -> u8 {
        self.size_y
    }
    fn line_iterators(&self) -> Vec<fn(u8, u8) -> Vec<Vec<Coords>>> {
        vec![
            make_rows_iterator,
            make_columns_iterator,
            make_diagonal_l_iterator,
            make_diagonal_r_iterator
        ]
    }
    fn lines(&self) -> Vec<Vec<Vec<Coords>>> {
        self.line_iterators().iter().map(|f| f(self.size_x(), self.size_y())).collect()
    }
}

// game "domain" logic
pub trait GameOperations<T: MatrixOperations = Self> {
    fn current_depth(&self) -> u8;
    fn max_depth(&self) -> u8;
    fn depth_left(&self) -> u8;
    fn next_player(&self) -> Result<Player, String>;
    fn last_player(&self) -> Result<Player, String>;
    fn can_continue(&self) -> bool;
    fn try_winner(&self) -> Cell;
    fn is_finished(&self) -> bool;
    fn is_stalemate(&self) -> bool;
    fn possible_moves(&self) -> Vec<Move>;
    fn is_turn_winning(&self, turn: &Turn) -> bool;
}

// todo really, iterators
fn make_rows_iterator(width: u8, height: u8) -> Vec<Vec<Coords>> {
    let mut rows = Vec::new();
    for y in 0..height {
        let mut row = Vec::new();
        for x in 0..width {
            row.push((x, y));
        }
        rows.push(row);
    }
    rows
}

fn make_columns_iterator(width: u8, height: u8) -> Vec<Vec<Coords>> {
    let mut columns = Vec::new();
    for x in 0..width {
        let mut column = Vec::new();
        for y in 0..height {
            column.push((x, y));
        }
        columns.push(column);
    }
    columns
}

fn make_diagonal_l_iterator(width: u8, height: u8) -> Vec<Vec<Coords>> {
    let mut diagonals = Vec::new();
    for k in 0..(width + height - 1) {
        let mut diagonal = Vec::new();
        for j in 0..(k + 1) {
            let i: u8 = k - j;
            if i < height && j < width {
                diagonal.push((i, j));
            }
        }
        diagonals.push(diagonal);
    }
    diagonals
}

fn make_diagonal_r_iterator(width: u8, height: u8) -> Vec<Vec<Coords>> {
    let mut diagonals = Vec::new();
    for k in 0..(width + height - 1) {
        let mut diagonal = Vec::new();
        for j in 0..(k + 1) {
            let i: u8 = k - j;
            if i < height && j < width {
                diagonal.push((i, height - j - 1));
            }
        }
        diagonals.push(diagonal);
    }
    diagonals
}


impl GameOperations for State {
    fn current_depth(&self) -> u8 {
        self.coords_history.len() as u8
    }
    fn depth_left(&self) -> u8 {
        self.max_depth() - self.current_depth()
    }
    fn max_depth(&self) -> u8 {
        self.size_x * self.size_y
    }
    fn next_player(&self) -> Result<Player, String> {
        if !self.can_continue() {
            return Err("Game is over".into());
        }
        Ok(match self.coords_history.last() {
            Some(&(x, y)) => match self.field[self.calc_field_index(x, y) as usize] {
                Some(Player::Red) => Player::Blue,
                Some(Player::Blue) => Player::Red,
                _ => panic!("next_player: unexpected cell state")
            }
            None => FIRST_PLAYER
        })
    }
    fn last_player(&self) -> Result<Player, String> {
        self.coords_history.last().map(|&(x, y)| {
            self.field[self.calc_field_index(x, y) as usize].unwrap()
        }).ok_or("No moves yet".into())
    }
    fn can_continue(&self) -> bool {
        !self.is_finished() && !self.is_stalemate()
    }
    fn try_winner(&self) -> Cell {
        self.winner_cache
    }
    fn is_finished(&self) -> bool {
        self.try_winner().is_some()
    }
    fn is_stalemate(&self) -> bool {
        !self.is_finished() && self.size_x as usize * self.size_y as usize == self.coords_history.len()
    }
    fn possible_moves(&self) -> Vec<Move> {
        if self.is_finished() {
            return Vec::new();
        }
        if self.is_stalemate() {
            return Vec::new();
        }
        let mut res = Vec::new();
        let player = self.next_player().unwrap();
        for i in 0..self.size_y {
            // "better turns first" order, where the positions at the middle are prioritized https://github.com/PascalPons/connect4/commit/6caf32a4845bf1478b0d30bebd6366bfea75b7b5
            let y = (self.size_y as i8) / 2 + (1-2 * (i % 2) as i8) * ( i as i8 + 1 ) / 2;
            for s in vec![Side::Left, Side::Right].into_iter() {
                if self.validate_turn((player, y.clone() as u8, s.clone())).is_ok() {
                    res.push((y as u8, s))
                }
            }
        }
        res
    }
    fn is_turn_winning(&self, turn: &Turn) -> bool {
        let (player, height, side) = turn.clone();
        let mut bruteforce_game = self.clone();
        let move_result = bruteforce_game.push((player, height, side));
        if move_result.is_err() {
            return false;
        }
        bruteforce_game.try_winner().is_some()
    }
}

pub trait GameSerializations<T: MatrixOperations = Self> {
    fn serialize(&self) -> GameStateSerialized;
    fn hash_non_historical(&self) -> String;
    fn deserialize(s: &GameStateSerialized) -> Result<T, String>;
    fn to_rows(&self) -> Vec<Vec<Option<Player>>>; // for network, keep here or...?
}

trait WinnerCache<T: GameOperations + MatrixOperations = Self> {
    fn try_winner_(&mut self) -> Cell;
}

impl WinnerCache for State {
    fn try_winner_(&mut self) -> Cell {
        let last_move_ = self.coords_history.last();
        if last_move_.is_none() {
            return Cell::None;
        }
        let last_move = last_move_.unwrap();
        fn check_line(state: &State, player: Player, lc: &Coords, rc: &Coords, acc: u8, lplus: &dyn Fn(&Coords) -> Option<Coords>, rplus: &dyn Fn(&Coords) -> Option<Coords>) -> bool {
            if acc == WIN_LEN {
                return true;
            }
            let next_lc = lplus(lc);
            let next_rc = rplus(rc);
            if next_lc.is_none() && next_rc.is_none() {
                return false;
            }
            let next_lc_player_me: bool = next_lc.and_then(|c| state.field[state.calc_field_index(c.0, c.1) as usize]).map(|p| p == player).unwrap_or(false);
            let next_rc_player_me: bool = next_rc.and_then(|c| state.field[state.calc_field_index(c.0, c.1) as usize]).map(|p| p == player).unwrap_or(false);
            return if next_lc_player_me {
                check_line(state, player, &next_lc.unwrap(), &rc, acc + 1, lplus, rplus)
            } else if next_rc_player_me {
                check_line(state, player, &lc, &next_rc.unwrap(), acc + 1, lplus, rplus)
            } else {
                false
            }
        }
        fn checked_coords(c: (i8, i8), state: &State) -> Option<Coords> {
            if c.0 < 0 || c.0 >= state.size_x as i8 || c.1 < 0 || c.1 >= state.size_y as i8 {
                return None;
            }
            Some((c.0 as u8, c.1 as u8))
        }
        let horizontal_adders = (|c: &Coords| checked_coords((c.0 as i8 - 1, c.1 as i8), &self), |c: &Coords| checked_coords((c.0 as i8 + 1, c.1 as i8), &self));
        let vertical_adders = (|c: &Coords| checked_coords((c.0 as i8, c.1 as i8 - 1), &self), |c: &Coords| checked_coords((c.0 as i8, c.1 as i8 + 1), &self));
        let diag_l_r_adders = (|c: &Coords| checked_coords((c.0 as i8 - 1, c.1 as i8 + 1), &self), |c: &Coords| checked_coords((c.0 as i8 + 1, c.1 as i8 - 1), &self));
        let diag_r_l_adders = (|c: &Coords| checked_coords((c.0 as i8 - 1, c.1 as i8 - 1), &self), |c: &Coords| checked_coords((c.0 as i8 + 1, c.1 as i8 + 1), &self));
        let fns: Vec<(Box<dyn Fn(&Coords) -> Option<Coords>>, Box<dyn Fn(&Coords) -> Option<Coords>>)> = vec![
            (Box::new(horizontal_adders.0), Box::new(horizontal_adders.1)),
            (Box::new(vertical_adders.0), Box::new(vertical_adders.1)),
            (Box::new(diag_l_r_adders.0), Box::new(diag_l_r_adders.1)),
            (Box::new(diag_r_l_adders.0), Box::new(diag_r_l_adders.1)),
        ];
        for adder in fns.iter() {
            let (l, r) = adder;
            if check_line(&self, self.last_player().unwrap(), &last_move, &last_move, 1, &**l, &**r) {
                return self.last_player().unwrap().into();
            }
        }

        None
    }

}

fn validate_continuous<T: Copy>(v: &Vec<Option<T>>) -> Result<Vec<T>, String> {
    if v.len() == 0 { return Ok(vec![]); }
    let bools = v.iter().map(|x| x.is_some()).collect::<Vec<bool>>();
    let (nempties, empties): (Vec<_>, Vec<_>) = v.iter().partition(|&x| x.is_some());
    let valid = nempties.iter().chain(empties.iter()).map(|x: &Option<T>| x.is_some()).collect::<Vec<bool>>();
    if !bools.eq(valid.as_slice()) {return Err("invalid continuous".into());}
    Ok(nempties.iter().map(|x: &Option<T>| x.unwrap()).collect::<Vec<T>>())
}

// TODO tie to GameSerializations somehow
// check width / height are consistent
fn validate_serialized_dimensions(s: &String) -> Result<(u8, u8), String> {
    if s.len() == 0 { return Err("empty game?".into()); }
    let row_strings = s.trim().split(SERIALIZATION_ROW_SEPARATOR).collect::<Vec<&str>>();
    let height = row_strings.len();
    let width = split_row(row_strings[0]).collect::<Vec<&str>>().len();
    if height < MIN_DIM as usize || width < MIN_DIM as usize { return Err("invalid dimensions".into()); }
    for row_string in row_strings {
        let len = split_row(row_string).count();
        if len != width {
            return Err(format!("invalid dimensions for one of the rows, expected {}, got {}", width, len).into());
        }
    }
    Ok((width as u8, height as u8))
}

// TODO tie to GameSerializations somehow as private
// build a coords/player consequent turn order from a serialized string to fill up state conveniently
fn deserialize_intermediate_history(s: &String) -> Result<Vec<(Coords, Player)>, String> {
    let (width, height) = validate_serialized_dimensions(s)?;

    let mut history: Vec<Option<(Coords, Player)>> = vec![None; height as usize * width as usize];
    for (y, line) in s.trim().split(SERIALIZATION_ROW_SEPARATOR).enumerate() {
        for (x, sturn) in split_row(line.trim()).filter(|x| x.len() != 0).enumerate() {
            let nturn = sturn.to_string().parse::<u8>().unwrap(); // 1-indexed
            if nturn == 0 { continue; }
            // here we can assume that odds iturn are red, evens are blue
            let player = if nturn % 2 == 0 { Player::Blue } else { Player::Red };
            let prev_h = &history[(nturn - 1) as usize];
            if prev_h.is_some() { return Err(format!("duplicate turn {}", nturn)); }
            history[(nturn - 1) as usize] = Some(((x as u8, y as u8), player));
        }
    }
    let consecutive = validate_continuous(&history)?;
    Ok(consecutive)
}

impl GameSerializations for State {

    fn serialize(&self) -> GameStateSerialized {
        let mut field: Vec<u8> = vec![0; self.size_x as usize * self.size_y as usize];
        for (hi, coords) in self.coords_history.iter().enumerate() {
            let i = self.calc_field_index(coords.0, coords.1);
            field[i as usize] = hi as u8 + 1; // serialized turns are 1-indexed
        }
        return GameStateSerialized(field.chunks(self.size_x as usize).map(|x| x.iter().map(|n|n.to_string()).collect::<Vec<String>>().join(SERIALIZATION_COL_SEPARATOR))
            .collect::<Vec<String>>().join(SERIALIZATION_ROW_SEPARATOR));
    }
    fn hash_non_historical(&self) -> String {
        let mut field: Vec<Cell> = vec![None; self.size_x as usize * self.size_y as usize];
        for (j, coords) in self.coords_history.iter().enumerate() {
            let i = self.calc_field_index(coords.0, coords.1);
            field[i as usize] = Some(if j % 2 == 0 { Red } else { Blue });
        }
        return field.chunks(self.size_x as usize).map(|x| x.iter().map(|n| (match n {
            Some(Blue) => "B",
            Some(Red) => "R",
            None => "_",
        }).to_string()).collect::<Vec<String>>().join(SERIALIZATION_COL_SEPARATOR))
            .collect::<Vec<String>>().join(SERIALIZATION_ROW_SEPARATOR);
    }

    fn deserialize(s: &GameStateSerialized) -> Result<State, String> {
        let (width, height) = validate_serialized_dimensions(&s.0)?;
        let mut state = State::new(width, height);
        for (coords, player) in deserialize_intermediate_history(&s.0)?.iter() {
            state.coords_history.push(coords.clone());
            let index = calc_field_index(width, coords.0, coords.1);
            state.field[index as usize] = Some(player.clone());
        }
        state.update_winner();
        Ok(state)
    }
    fn to_rows(&self) -> Vec<Vec<Option<Player>>> {
        let mut res = vec![vec![None; self.size_x as usize]; self.size_y as usize];
        for (i, cell) in self.field.iter().enumerate() {
            let coords = calc_coords(self.size_x, i as u8);
            res[coords.1 as usize][coords.0 as usize] = cell.clone();
        }
        res
    }
}

impl State {

    fn validate_turn(&self, turn: Turn) -> Result<(), String> {
        if !self.can_continue() {
            return Err("Game is over".into());
        }
        if &self.next_player()? != &turn.0 {
            return Err("Wrong player".into());
        }
        let next = self.next_cell_towards(turn.2.clone(), turn.1 as u8)?;
        if next.is_none() {
            return Err(format!("Can't push turn {} {} {}", turn.0, turn.1, turn.2));
        }
        Ok(())
    }
    pub fn push_move(&mut self, move_: Move) -> Result<(), String> {
        let player = self.next_player()?;
        let turn = (player, move_.0, move_.1);
        self.push(turn)
    }
    fn update_winner(&mut self) -> () {
        self.winner_cache = self.try_winner_();
    }
    pub fn push(&mut self, turn: Turn) -> Result<(), String> {
        self.validate_turn(turn)?;
        let next = self.next_cell_towards(turn.2.clone(), turn.1 as u8)?;
        // self.history.push(turn.clone());
        self.coords_history.push(next.unwrap());
        let next_ = next.unwrap(); // already checked above
        let index =  self.calc_field_index(next_.0.clone(), next_.1.clone()) as usize;
        self.field[index] = Some(turn.0);
        self.update_winner();
        Ok(())
    }
    pub fn pop(&mut self) -> Result<(), String> {
        // let turn = self.history.pop().ok_or_else(|| String::from("No turns to pop"))?;
        let coords = self.coords_history.pop().ok_or_else(|| String::from("No turns to pop"))?;
        let index = self.calc_field_index(coords.0, coords.1) as usize;
        self.field[index] = None;
        self.winner_cache = None;
        Ok(())
    }
    pub fn new(size_x: u8, size_y: u8) -> State {
        let size_xy = size_x as usize * size_y as usize;
        State { size_x, size_y, coords_history: Vec::with_capacity(size_xy), field: vec![None; size_xy], winner_cache: None }
    }
}


#[cfg(test)]
mod tests {
    use crate::db::GameStateSerialized;
    use crate::game::{calc_field_index, GameOperations, Move};
    use crate::game::GameSerializations;
    use crate::game::Player::*;
    use crate::game::Side::{Left, Right};

    // players are taking turns exclusively in the middle of the board, red going only left, blue going only right
    const GAME_NAIVE_HORIZONTAL_WON: &str = r#"
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
1 3 5 7 6 4 2
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
    "#;
    const GAME_NAIVE_VERTICAL_WON: &str = r#"
1 0 0 0 0 0 2
3 0 0 0 0 0 4
5 0 0 0 0 0 6
7 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
    "#;
    const GAME_VERTICAL_BLUE_WON: &str = r#"
1 0 0 0 0 0 2
3 0 0 0 0 0 4
5 0 0 0 0 0 6
0 0 0 0 0 0 8
7 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
    "#;
    const GAME_DIAGONAL_RED_WON: &str = r#"
1 0 0 0  0  0 0
2 3 0 0  0  0 0
4 6 7 0  0  0 0
0 0 0 11 10 9 8
0 0 0 0  0  0 0
0 0 0 0  0  0 0
0 0 0 0  0  0 5
    "#;
    const GAME_DIAGONAL_ALTERNATIVE_RED_WON: &str = r#"
6 8 10 11 0 0 0
4 5 7  0  0 0 0
2 3 0  0  0 0 0
1 0 0  0  0 0 0
9 0 0  0  0 0 0
0 0 0  0  0 0 0
0 0 0  0  0 0 0
    "#;
    const GAME_ONGOING: &str = r#"
1  0  0  0  0 0  2
3  9  11 12 0 10 4
5  0  0  0  0 16 6
14 0  0  0  0 0 0
7  0  0  0  0 0 8
13 15 17 0  0 0 18
0  0  0  0  0 0 0
    "#;
    const GAME_CLOGGED: &str = r#"
1  0  0  0  0 0  2
3  9  11 12 19 10 4
5  23 22 21 20 16 6
14 0  0  0  0 0 0
7  0  0  0  0 0 8
13 15 17 0  0 0 18
0  0  0  0  0 0 0
    "#;
    const GAME_STALEMATE: &str = r#"
1 2
3 4
    "#;
    const GAME_BLUE_WINNING: &str = r#"
1 0 0 0 0 0 0
5 0 0 0 0 0 2
3 0 0 0 0 0 4
0 0 0 0 0 0 6
0 0 0 0 0 0 7
0 0 0 0 0 0 0
0 0 0 0 0 0 0
    "#;
    const GAME_NOT_SQUARE: &str = r#"
1 0 0 0 0 0
5 0 0 0 0 2
3 0 0 0 0 4
0 0 0 0 0 6
0 0 0 0 0 7
0 0 0 0 0 0
0 0 0 0 0 0
    "#;
    const GAME_EMPTY: &str = r#"
0 0 0 0 0
0 0 0 0 0
0 0 0 0 0
0 0 0 0 0
0 0 0 0 0
    "#;
    // when a "column" (or a "row") isn't reset properly
    // it could cause a column square to be counted into row squares or vice versa
    const GAME_WINNER_ALGORITHM_BUG_1: &str = r#"
15 13 12 7 3  2  1
11 10 9  8 6  5  4
16 18 20 0 19 17 14
0  0  0  0 0  0  0
0  0  0  0 0  0  0
0  0  0  0 0  0  0
0  0  0  0 0  0  0
    "#;
    #[test]
    fn calc_field_index_rect_test() {
        assert_eq!(calc_field_index(3,  1, 1), 4);
        assert_eq!(calc_field_index(3,  2, 1), 5);
        assert_eq!(calc_field_index(4,  1, 1), 5);
    }
    #[test]
    fn serializations() {
        let state = super::State::deserialize(&GameStateSerialized(GAME_NAIVE_HORIZONTAL_WON.to_string())).unwrap();
        assert_eq!(GAME_NAIVE_HORIZONTAL_WON.to_string().trim(), state.serialize().0)
    }
    #[test]
    fn winner_horizontal() {
        let state = super::State::deserialize(&GameStateSerialized(GAME_NAIVE_HORIZONTAL_WON.to_string())).unwrap();
        assert!(!state.is_stalemate());
        assert_eq!(Some(Red), state.try_winner())
    }
    #[test]
    fn winner_vertical() {
        let state = super::State::deserialize(&GameStateSerialized(GAME_NAIVE_VERTICAL_WON.to_string())).unwrap();
        assert!(!state.is_stalemate());
        assert_eq!(Some(Red), state.try_winner())
    }
    #[test]
    fn winner_diagonal() {
        let state = super::State::deserialize(&GameStateSerialized(GAME_DIAGONAL_RED_WON.to_string())).unwrap();
        assert_eq!(Some(Red), state.try_winner())
    }
    #[test]
    fn winner_diagonal_alternative() {
        let state = super::State::deserialize(&GameStateSerialized(GAME_DIAGONAL_ALTERNATIVE_RED_WON.to_string())).unwrap();
        assert_eq!(Some(Red), state.try_winner())
    }
    #[test]
    fn winner_vertical_blue() {
        let state = super::State::deserialize(&GameStateSerialized(GAME_VERTICAL_BLUE_WON.to_string())).unwrap();
        assert!(!state.is_stalemate());
        assert_eq!(Some(Blue), state.try_winner())
    }
    #[test]
    fn game_ongoing() {
        let state = super::State::deserialize(&GameStateSerialized(GAME_ONGOING.to_string())).unwrap();
        assert!(!state.is_stalemate());
        assert!(!state.is_finished());
        assert_eq!(None, state.try_winner())
    }
    #[test]
    fn game_stalemate() {
        let state = super::State::deserialize(&GameStateSerialized(GAME_STALEMATE.to_string())).unwrap();
        assert!(state.is_stalemate())
    }
    #[test]
    fn winning_turn() {
        let mut state = super::State::deserialize(&GameStateSerialized(GAME_BLUE_WINNING.to_string())).unwrap();
        assert!(!state.is_finished());
        state.push((Blue, 0, Right)).unwrap();
        assert!(state.is_finished());
        assert_eq!(Some(Blue), state.try_winner())
    }
    #[test]
    #[should_panic]
    fn not_a_square() {
        let mut state = super::State::deserialize(&GameStateSerialized(GAME_NOT_SQUARE.to_string())).unwrap();
        state.push((Blue, 0, Right)).unwrap();
    }
    #[test]
    fn bug1() {
        let state = super::State::deserialize(&GameStateSerialized(GAME_WINNER_ALGORITHM_BUG_1.to_string())).unwrap();
        assert_eq!(None, state.try_winner())
    }
    #[test]
    fn many_turns() {
        let mut state = super::State::deserialize(&GameStateSerialized(GAME_EMPTY.to_string())).unwrap();
        state.push((Red, 0, Right)).unwrap();
        state.push((Blue, 1, Right)).unwrap();
        state.push((Red, 0, Right)).unwrap();
        state.push((Blue, 1, Right)).unwrap();
        state.push((Red, 0, Right)).unwrap();
        state.push((Blue, 1, Right)).unwrap();
        assert!(!state.is_finished());
        state.push((Red, 0, Right)).unwrap();
        assert!(state.is_finished());
    }
    #[test]
    fn possible_moves_empty() {
        let state = super::State::deserialize(&GameStateSerialized(GAME_EMPTY.to_string())).unwrap();
        assert_eq!(vec![(2, Left), (2, Right), (1, Left), (1, Right), (3, Left), (3, Right), (0, Left), (0, Right), (4, Left), (4, Right)], state.possible_moves());
    }
    #[test]
    fn possible_moves_nogame() {
        let state = super::State::deserialize(&GameStateSerialized(GAME_STALEMATE.to_string())).unwrap();
        let v: Vec<Move> = Vec::new();
        assert_eq!(v, state.possible_moves());
    }
    #[test]
    fn possible_moves_cloggedgame() {
        let state = super::State::deserialize(&GameStateSerialized(GAME_CLOGGED.to_string())).unwrap();
        assert_eq!(vec![(3, Left), (3, Right), (4, Left), (4, Right), (5, Left), (5, Right), (0, Left), (0, Right), (6, Left), (6, Right)], state.possible_moves());
    }
    #[test]
    fn hashing() {
        let state = super::State::deserialize(&GameStateSerialized(r#"
0 0 0 0 0
0 0 0 0 4
0 0 7 5 3
0 0 0 2 1
0 0 0 0 6
    "#.to_string())).unwrap();
        assert_eq!(state.hash_non_historical(), "_ _ _ _ _\n_ _ _ _ B\n_ _ R R R\n_ _ _ B R\n_ _ _ _ B");
    }
}
