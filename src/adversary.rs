use crate::broker::SimpleBroker;
use crate::db::{fetch_game_state};
use crate::db_schema::DbGame;
use crate::game::{GameOperations, GameSerializations, Move, Player, State};
use futures_util::StreamExt;
use rand::prelude::SliceRandom;
use crate::adversary_minimax::{minimax, MINMAX_DEPTH_RESTRICTION};
use crate::db::update_game_state;

#[derive(Debug, Clone, Copy, DbEnum, Eq, PartialEq, async_graphql::Enum)]
#[DbValueStyle = "SCREAMING_SNAKE_CASE"]
pub enum BotId {
    RANDY, SMART
}

impl From<String> for BotId {
    fn from(input: String) -> Self {
        match input.as_str() {
            "RANDY" => BotId::RANDY,
            "SMART" => BotId::SMART,
            _ => panic!("Unknown bot id"),
        }
    }
}

pub async fn run_subscribe_bots() {
    SimpleBroker::<DbGame>::subscribe().for_each(|g| async move { try_bot(&g).await }).await;
}

fn bot_can_move(db_game: &DbGame) -> bool {
    if db_game.bot_id.is_none() {
        return false;
    }
    // only one player
    let adversaries = vec![db_game.player_red.as_ref(), db_game.player_blue.as_ref()].into_iter().filter(|p| p.is_some()).map(|p| p.unwrap()).collect::<Vec<_>>();
    if adversaries.len() != 1 {
        return false;
    }
    // let adversary = &adversaries[0];
    let state = State::deserialize(&db_game.state).unwrap();
    if state.next_player().is_err() {
        return false;
    }
    let player = state.next_player().unwrap();
    let slot_empty = match player {
        Player::Red => {
            db_game.player_red == None
        }
        Player::Blue => {
            db_game.player_blue == None
        },
    };
    if !slot_empty {
        return false;
    }
    return true;
}

pub async fn try_bot(db_game: &DbGame) {
    if !bot_can_move(db_game) {
        return;
    }
    let bot_id = BotId::try_from(db_game.bot_id.clone().unwrap()).unwrap();
    let mut state = State::deserialize(&db_game.state).unwrap();
    let player = state.next_player().unwrap();

    let bmove = bot_move(&bot_id, &state);

    match bmove {
        Some(m) => {
            state.push((player, m.0, m.1)).unwrap(); // there is a possible move, safe to unwrap
            update_game_state(&db_game.id, state.serialize()).await.unwrap();
        }
        None => {
            return;
        }
    };
}

fn randy(game: &State) -> Option<Move> {
    let mut rng = rand::thread_rng();
    let mut actions = game.possible_moves();
    actions.shuffle(&mut rng);
    actions.pop()
}

fn bot_move(bot_id: &BotId, game: &State) -> Option<Move> {
    match bot_id {
        BotId::RANDY => {
            randy(game)
        }
        BotId::SMART => {
            return match game.next_player() {
                Err(_) => {
                    None
                }
                Ok(player) => {
                    // first 2 turns are for Randy
                    if game.current_depth() < 2 {
                        randy(game)
                    } else if game.depth_left() > MINMAX_DEPTH_RESTRICTION {
                        randy(game)
                    } else {
                        minimax(&mut game.clone())
                    }

                }
            }
        }
    }
}

