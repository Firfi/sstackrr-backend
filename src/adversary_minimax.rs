use std::cmp::{max, min};
use std::collections::HashMap;
use crate::db::GameStateSerialized;
use crate::game::{Coords, GameOperations, GameSerializations, MatrixOperations, Move, Player, State, WIN_LEN};
use moka::unsync::Cache;

pub const MINMAX_DEPTH_RESTRICTION: u8 = 15;

pub (crate) fn minimax(game: &State) -> Option<Move> {
    if game.next_player().is_err() {
        return None;
    }
    let mut hm = Cache::new(10000);
    let weak = false;
    minimax_recursion(&mut game.clone(), game.next_player().unwrap(), &mut hm, if weak { -1 } else { game.size_x() as i32 * game.size_y() as i32 / 2 * -1 }, if weak { 1 } else { game.size_x() as i32 * game.size_y() as i32 / 2 }, Some(MINMAX_DEPTH_RESTRICTION)).0
}

// collect potential scores per 4-cell windows, weighting extremes up
fn expectimax(game: &State, player: Player) -> i32 {
    fn window_score(game: &State, player: Player, window: &[Coords]) -> i32 {
        let mut occurrences: i32 = 0;
        let mut last_player: Option<Player> = None;
        let mut non_homogenous = false;
        for c in window {
            let cell = game.get_cell(c.0, c.1).unwrap();
            if cell == None {
                continue;
            }
            if last_player.is_some() && last_player != cell {
                non_homogenous = true;
                break;
            }
            if last_player == None {
                last_player = cell;
            }
            occurrences += 1;
        }
        // todo better score function
        if non_homogenous {
            return 0;
        }
        if last_player.is_none() {
            return 0;
        }
        let signum = if player == last_player.unwrap() { 1 } else { -1 };
        // weight up "one left to win" considerably
        if occurrences == 3 {
            return 30 * signum;
        }
        if occurrences == 2 {
            return 4 * signum;
        }
        return occurrences * signum;
    }
    let score = game.lines().iter().map(|lines| lines.concat()).map(|line| {
        let windows = line.windows(WIN_LEN as usize);
        windows.map(|window| {
            window_score(game, player, window)
        }).sum::<i32>()
    }).sum();
    min(max(score, -1 as i32 * game.size_y() as i32 * game.size_x() as i32 - 1), game.size_y() as i32 * game.size_x() as i32 + 1) // todo really scale up/down to game size i.e. https://stackoverflow.com/questions/5294955/how-to-scale-down-a-range-of-numbers-with-a-known-min-and-max-value
}

fn minimax_recursion(game: &mut State, player: Player,
                     solutions_done: &mut Cache<String, (Option<Move>, Option<i32>)>,
                     mut alpha: i32,
                     mut beta: i32,
                     recommended_depth: Option<u8>) -> (Option<Move>, Option<i32>) {
    if recommended_depth.is_some() && recommended_depth.unwrap() == 0 {
        return (None, None);
    }
    let mut possible_moves = game.possible_moves(); // so the caller won't trick us with a wrong depth
    if possible_moves.is_empty() {
        // last player supposed to be here when possible_moves is exhausted
        return (None, Some(0));
    }
    if game.is_stalemate() {
        return (None, Some(0));
    }
    // try a winning move
    for m in possible_moves.iter() {
        game.push_move(m.clone()).unwrap();
        let is_win = game.try_winner().is_some();
        game.pop().unwrap();

        if is_win {
            return (Some(m.clone()),
                    Some((game.size_x() as i32 * game.size_y() as i32 + 1 - game.current_depth() as i32) / 2));
        }
    }

    let mut max = (game.size_x() as i32 * game.size_y() as i32 - 1 - game.current_depth() as i32) / 2;
    if beta > max {
        beta = max; // there is no need to keep beta above our max possible score.
        if alpha >= beta {
            return (None, Some(beta));
        } // prune the exploration if the [alpha;beta] window is empty.
    }

    // TODO here goes the turn selection optimisation
    // possible_moves.sort_by(|a, b| {
    //     game.push_move(a.clone());
    //     let a_score = expectimax(game, player);
    //     game.pop();
    //     game.push_move(b.clone());
    //     let b_score = expectimax(game, player);
    //     game.pop();
    //     a_score.cmp(&b_score)
    // });
    let mut best_move: Option<Move> = None;
    for m in possible_moves.into_iter() {
        let hash_pref = game.serialize();
        game.push_move(m).unwrap();
        let hash = game.hash_non_historical();
        let score = if solutions_done.contains_key(&hash) {
            solutions_done.get(&hash).unwrap().1
        } else {
            let r = minimax_recursion(game, player, solutions_done, beta * -1, alpha * -1, recommended_depth.map(|d| d - 1)).1.map(|s| s * -1);
            solutions_done.insert(hash, (Some(m.clone()), r.clone()));
            r
        };

        if score.is_some() && score.unwrap() >= beta {
            let res = (Some(m.clone()), score);
            game.pop().unwrap();
            return res;
        }
        if score.is_some() && score.unwrap() > alpha {
            alpha = score.unwrap();
            best_move = Some(m.clone());
        }
        game.pop().unwrap();
    }


    (best_move, Some(alpha))

}

#[cfg(test)]
mod tests {
    use crate::adversary_minimax::{expectimax, minimax};
    use crate::db::GameStateSerialized;
    use crate::game::Side::{Left, Right};
    use crate::game::{GameSerializations, State};
    use crate::game::Player::{Blue, Red};

    const GAME_OPPORTUNITY: &str = r#"
1 9 8  2
3 4 10 11
5 6 7  12
0 0 14  13
    "#;
    const GAME_OPPORTUNITY2: &str = r#"
0 0 0 0 0
0 0 8 6 2
0 0 4 3 1
0 0 0 7 5
0 0 0 0 9
    "#;
    const GAME_BLOCKER: &str = r#"
8 9 10 11 0
0 0 0  0  4
0 0 7  5  3
0 0 0  2  1
0 0 0  0  6
    "#;
    const GAME_OPPORTUNITY_BIGGER: &str = r#"
1 0 0 2
3 4 0 0
5 6 0 0
0 0 0 0
    "#;
    const GAME_OPPORTUNITY_REAL: &str = r#"
0 0 0 0 6
0 1 0 0 2
0 0 3 0 0
0 0 0 0 4
0 0 0 0 5
    "#;
    const GAME_EMPTY: &str = r#"
0 0 0 0 0
0 0 0 0 0
0 0 0 0 0
0 0 0 0 0
0 0 0 0 0
    "#;
    const GAME_WINNING_WINDOW: &str = r#"
0 0 0 0 0 0 0
0 0 0 0 0 0 0
4 0 0 0 0 0 3
2 0 0 0 0 5 1
0 0 0 0 0 0 0
6 0 0 0 0 0 0
0 0 0 0 0 0 0
    "#;
    const GAME_WINNING_WINDOWS: &str = r#"
0 0 0 0 0 0 0
0 0 0 0 0 0 0
4 0 0 0 0 0 3
2 0 0 0 0 0 1
0 0 0 0 0 0 5
6 0 0 0 0 0 0
0 0 0 0 0 0 0
    "#;
    // with more optimisations, uncover more 0s!
    const PERFORMANCE_TEST: &str = r#"
0 0 0 0
0 0 0 0
0 0 0 0
0 0 0 1
    "#;
    // when bot returns a turn None for that one
    const BUG_1: &str = r#"
0 0 0 0
1 2 0 0
4 0 0 0
3 0 0 7
5 6 0 0
    "#;
    #[test]
    fn expectimax_none() {
        let game = &State::deserialize(&GameStateSerialized(GAME_EMPTY.to_string())).unwrap();
        assert_eq!(expectimax(game, Red), 0);
    }
    #[test]
    fn expectimax_three_better_than_many_two() {
        // when there are 3 already on a winning window, many of 2s of the other player aren't better
        let game = &State::deserialize(&GameStateSerialized(GAME_WINNING_WINDOW.to_string())).unwrap();
        assert_eq!(expectimax(game, Red).signum(), -1);
    }
    #[test]
    fn expectimax_winning_windows_aint_equal() {
        // a winning sequence that goes into several winning windows will be stronger
        let game = &State::deserialize(&GameStateSerialized(GAME_WINNING_WINDOWS.to_string())).unwrap();
        assert_eq!(expectimax(game, Red).signum(), 1);
    }
    #[test]
    fn minimax_opportunity() {
        let r = minimax(&State::deserialize(&GameStateSerialized(GAME_OPPORTUNITY.to_string())).unwrap());
        assert_eq!(r, Some((3, Left)));
    }
    #[test]
    fn minimax_opportunity2() {
        let r = minimax(&State::deserialize(&GameStateSerialized(GAME_OPPORTUNITY2.to_string())).unwrap());
        assert_eq!(r, Some((1, Right)));
    }
    #[test]
    fn minimax_blocker() {
        let r = minimax(&State::deserialize(&GameStateSerialized(GAME_BLOCKER.to_string())).unwrap());
        assert_eq!(r, Some((2, Right)));
    }
    #[test]
    fn minimax_bigger_opportunity() {
        let r = minimax(&State::deserialize(&GameStateSerialized(GAME_OPPORTUNITY_BIGGER.to_string())).unwrap());
        assert_eq!(r, Some((3, Left)));
    }
    #[test]
    fn minimax_real_opportunity() {
        let r = minimax(&State::deserialize(&GameStateSerialized(GAME_OPPORTUNITY_REAL.to_string())).unwrap());
        assert_eq!(r, Some((3, Right)));
    }
    #[test]
    fn performance() {
        let r = minimax(&State::deserialize(&GameStateSerialized(PERFORMANCE_TEST.to_string())).unwrap());
    }
    #[test]
    fn bug_1() {
        let r = minimax(&State::deserialize(&GameStateSerialized(BUG_1.to_string())).unwrap());
        assert_eq!(r, Some((3, Left)));
    }
}