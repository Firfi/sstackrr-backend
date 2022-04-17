use crate::db::{DbGameAndPlayer, claim_game_player, fetch_game_state_for_player, GameStateSerialized, GameToken, init_game_state, PlayerToken, update_game_state, fetch_game_state};
use crate::game::{GameOperations, Player, Side, State};
use crate::game::GameSerializations;
use async_graphql::{FieldResult, Object, SimpleObject, InputObject, Schema, Subscription};
use async_graphql::futures_util::Stream;
use tokio_stream::StreamExt;
use crate::broker::SimpleBroker;
use crate::db_schema::DbGame;

#[derive(SimpleObject)]
pub struct GameStateResult {
    id: String,
    state: Vec<Vec<Option<Player>>>,
    next_player: Option<Player>,
    winner: Option<Player>,
    is_stalemate: bool,
}

#[derive(SimpleObject)]
pub struct ClaimPlayerResult {
    game: GameStateResult,
    player_token: String,
}

impl GameStateResult {
    pub fn from_game(id: GameToken, game: State) -> GameStateResult {
        GameStateResult {
            id: id.0.to_string(),
            state: game.to_rows(),
            next_player: if game.is_finished() || game.is_stalemate() { None } else { Some(game.next_player().unwrap()) },
            winner: game.try_winner(),
            is_stalemate: game.is_stalemate()
        }
    }
}

pub(crate) struct QueryRoot;

fn game_from_db_game(db_game: &DbGame) -> Result<State, String> {
    let serialized = db_game.state.clone();
    State::deserialize(GameStateSerialized(serialized))
}

fn game_state_result_from_db_game(db_game: &DbGame) -> Result<GameStateResult, String> {
    Ok(GameStateResult::from_game(GameToken(db_game.id.to_string()), game_from_db_game(db_game)?))
}

#[Object]
impl QueryRoot {
    pub(crate) async fn game(&self, game_token: String) -> FieldResult<GameStateResult> {
        Ok(game_state_result_from_db_game(&fetch_game_state(&GameToken(game_token)).await?)?)
    }
    pub(crate) async fn me(&self, player_token: String) -> FieldResult<Player> {
        Ok(fetch_game_state_for_player(&PlayerToken(player_token)).await?.player)
    }
}

pub(crate) type GraphQlSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

#[derive(InputObject)]
struct TurnInput {
    side: Side,
    height: u8,
}

pub(crate) struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn init_game(&self) -> FieldResult<GameStateResult> {
        Ok(game_state_result_from_db_game(&init_game_state().await?)?)
    }
    async fn claim_player(&self, game_token: String, player: Player) -> Result<ClaimPlayerResult, String> {
        let (id, db_game) = claim_game_player(&GameToken(game_token), player).await?;
        let game = game_state_result_from_db_game(&db_game)?;
        Ok((ClaimPlayerResult {
            player_token: id.to_string(),
            game,
        }))
    }
    async fn turn(&self, player_token: String, turn: TurnInput) -> Result<GameStateResult, String> {
        let player_token_parsed = PlayerToken(player_token);
        let db_game_and_player = fetch_game_state_for_player(&player_token_parsed).await?;
        let db_game = db_game_and_player.game;
        let player = db_game_and_player.player;
        let mut state = game_from_db_game(&db_game)?;
        state.push((player, turn.height, turn.side))?;
        update_game_state(&player_token_parsed, state.serialize()).await?;
        Ok(GameStateResult::from_game(GameToken(db_game.id.to_string()), state))
    }
}

pub(crate) struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    // a "readonly" game for anyone to subscribe to. I push the whole game state, because I'm lazy and also it isn't big size anyways
    async fn game(&self, game_token: String) -> impl Stream<Item = GameStateResult> {
        SimpleBroker::<DbGame>::subscribe().filter(move |db_game: &DbGame| {
            db_game.id.to_string() == game_token
        }).map(|db_game: DbGame| {
            game_state_result_from_db_game(&db_game).unwrap()
        })
    }
}