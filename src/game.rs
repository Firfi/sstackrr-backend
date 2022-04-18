// the main game file; for better testing and portability, it's immutable

use std::str::SplitWhitespace;
use strum_macros;
use async_graphql::Enum;
use crate::db::GameStateSerialized;

// code assumes our field is at least 1x1
const MIN_DIM: u8 = 1;
const WIN_LEN: u8 = 4;

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
pub type CoordsHistory = Vec<Coords>;
type Cell = Option<Player>;
type Field = Vec<Cell>;

#[derive(Clone, Debug)]
pub struct State {
    size_x: u8,
    size_y: u8,
    coords_history: CoordsHistory, // actually, we can do with Only this field
    field: Field // derivative to History+sizes but here for convenience and performance
}

type Coords = (u8, u8);

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
}

// game "domain" logic
pub trait GameOperations<T: MatrixOperations = Self> {
    fn next_player(&self) -> Result<Player, String>;
    fn can_continue(&self) -> bool;
    fn try_winner(&self) -> Cell;
    fn is_finished(&self) -> bool;
    fn is_stalemate(&self) -> bool;
}

impl GameOperations for State {
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
    fn can_continue(&self) -> bool {
        !self.is_finished() && !self.is_stalemate()
    }
    // big procedural blob... TODO we can potentially at least decouple x/y logic duplication
    // iterate through the field (both rows and columns together)
    fn try_winner(&self) -> Cell {

        // TODO this doesn't work for non-square fields
        for i in 0..self.size_x {
            // at least 2x2
            let mut current_x_player: Cell = None;
            let mut current_y_player: Cell = None;
            let mut current_x_count: u8 = 0;
            let mut current_y_count: u8 = 0;
            for j in 0..self.size_y {
                let rectangular_placeholder = None;
                let x_line = self.get_cell( i, j).unwrap_or(rectangular_placeholder);
                let y_line = self.get_cell( j, i).unwrap_or(rectangular_placeholder);
                if current_x_player.is_some() && x_line == current_x_player {
                    current_x_count += 1;
                } else {
                    current_x_player = x_line;
                    current_x_count = 1; // note that Optional goes into the count too but it's all right
                }
                if current_x_player.is_some() && current_x_count >= WIN_LEN {
                    return current_x_player;
                }
                if current_y_player.is_some() && y_line == current_y_player {
                    current_y_count += 1;
                } else {
                    current_y_player = y_line;
                    current_y_count = 1;
                }
                let x_won = current_x_player.is_some() && current_x_count >= WIN_LEN;
                let y_won = current_y_player.is_some() && current_y_count >= WIN_LEN;
                if x_won && y_won {
                    // something wrong with our data or computation; safety net:
                    panic!("x and y both won?");
                } else if x_won {
                    return current_x_player;
                } else if y_won {
                    return current_y_player;
                }
            }
        }
        None
    }
    fn is_finished(&self) -> bool {
        self.try_winner().is_some()
    }
    fn is_stalemate(&self) -> bool {
        !self.is_finished() && self.size_x as usize * self.size_y as usize == self.coords_history.len()
    }
}

pub trait GameSerializations<T: MatrixOperations = Self> {
    fn serialize(&self) -> GameStateSerialized;
    fn deserialize(s: &GameStateSerialized) -> Result<T, String>;
    fn to_rows(&self) -> Vec<Vec<Option<Player>>>; // for network, keep here or...?
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

    fn deserialize(s: &GameStateSerialized) -> Result<State, String> {
        let (width, height) = validate_serialized_dimensions(&s.0)?;
        let mut state = State::new(width, height);
        for (coords, player) in deserialize_intermediate_history(&s.0)?.iter() {
            state.coords_history.push(coords.clone());
            let index = calc_field_index(height, coords.0, coords.1);
            state.field[index as usize] = Some(player.clone());
        }
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

    pub fn push(&mut self, turn: Turn) -> Result<(), String> {
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
        // self.history.push(turn.clone());
        self.coords_history.push(next.unwrap());
        let next_ = next.unwrap(); // already checked above
        let index =  self.calc_field_index(next_.0, next_.1) as usize;
        self.field[index] = Some(turn.0);
        Ok(())
    }
    pub fn pop(&mut self) -> Result<(), String> {
        // let turn = self.history.pop().ok_or_else(|| String::from("No turns to pop"))?;
        let coords = self.coords_history.pop().ok_or_else(|| String::from("No turns to pop"))?;
        let index = self.calc_field_index(coords.0, coords.1) as usize;
        self.field[index] = None;
        Ok(())
    }
    pub fn new(size_x: u8, size_y: u8) -> State {
        let size_xy = size_x as usize * size_y as usize;
        State { size_x, size_y, coords_history: Vec::with_capacity(size_xy), field: vec![None; size_xy] }
    }
}

#[cfg(test)]
mod tests {
    use crate::game::GameOperations;
    use crate::game::GameSerializations;
    use crate::game::Player::*;
    use crate::game::Side::Right;

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
    const GAME_ONGOING: &str = r#"
1  0  0  0  0 0  2
3  9  11 12 0 10 4
5  0  0  0  0 16 6
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
    fn serializations() {
        let state = super::State::deserialize(GAME_NAIVE_HORIZONTAL_WON.to_string().into()).unwrap();
        assert_eq!(GAME_NAIVE_HORIZONTAL_WON.to_string().trim(), state.serialize().0)
    }
    #[test]
    fn winner_horizontal() {
        let state = super::State::deserialize(GAME_NAIVE_HORIZONTAL_WON.to_string().into()).unwrap();
        assert!(!state.is_stalemate());
        assert_eq!(Some(Red), state.try_winner())
    }
    #[test]
    fn winner_vertical() {
        let state = super::State::deserialize(GAME_NAIVE_VERTICAL_WON.to_string().into()).unwrap();
        assert!(!state.is_stalemate());
        assert_eq!(Some(Red), state.try_winner())
    }
    #[test]
    fn winner_vertical_blue() {
        let state = super::State::deserialize(GAME_VERTICAL_BLUE_WON.to_string().into()).unwrap();
        assert!(!state.is_stalemate());
        assert_eq!(Some(Blue), state.try_winner())
    }
    #[test]
    fn game_ongoing() {
        let state = super::State::deserialize(GAME_ONGOING.to_string().into()).unwrap();
        assert!(!state.is_stalemate());
        assert!(!state.is_finished());
        assert_eq!(None, state.try_winner())
    }
    #[test]
    fn game_stalemate() {
        let state = super::State::deserialize(GAME_STALEMATE.to_string().into()).unwrap();
        assert!(state.is_stalemate())
    }
    #[test]
    fn winning_turn() {
        let mut state = super::State::deserialize(GAME_BLUE_WINNING.to_string().into()).unwrap();
        assert!(!state.is_finished());
        state.push((Blue, 0, Right)).unwrap();
        assert!(state.is_finished());
        assert_eq!(Some(Blue), state.try_winner())
    }
    #[test]
    #[should_panic]
    fn not_a_square() {
        let mut state = super::State::deserialize(GAME_NOT_SQUARE.to_string().into()).unwrap();
        state.push((Blue, 0, Right)).unwrap();
    }
    #[test]
    fn bug1() {
        let mut state = super::State::deserialize(GAME_WINNER_ALGORITHM_BUG_1.to_string().into()).unwrap();
        assert_eq!(None, state.try_winner())
    }
    #[test]
    fn many_turns() {
        let mut state = super::State::deserialize(GAME_EMPTY.to_string().into()).unwrap();
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
}
