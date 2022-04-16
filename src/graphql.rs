use crate::db::{fetch_game_state, GameStateSerialized, GameToken, init_game_state, PlayerToken};
use crate::game::{GameOperations, Player, Side, State};
use crate::game::GameSerializations;
use async_graphql::{EmptyMutation, EmptySubscription, FieldResult, Object, SimpleObject, InputObject, Schema};

#[derive(SimpleObject)]
pub struct GameStateResult {
    id: String,
    state: Vec<Vec<Option<Player>>>,
    player: Option<Player>,
    winner: Option<Player>,
    is_stalemate: bool,
}

impl GameStateResult {
    pub fn from_game(id: GameToken, game: State) -> GameStateResult {
        GameStateResult {
            id: id.0.to_string(),
            state: game.to_rows(),
            player: if game.is_finished() || game.is_stalemate() { None } else { Some(game.next_player().unwrap()) },
            winner: game.try_winner(),
            is_stalemate: game.is_stalemate()
        }
    }
}

pub(crate) struct QueryRoot;

#[Object]
impl QueryRoot {
    pub(crate) async fn game(&self, player_token: String) -> FieldResult<GameStateResult> {
        let serialized = fetch_game_state(&PlayerToken(player_token)).await?;
        let game = State::deserialize(GameStateSerialized(serialized.state))?;
        Ok(GameStateResult::from_game(GameToken(serialized.id.to_string()), game))
    }
}

pub(crate) type GraphQlSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

#[derive(InputObject)]
struct Turn {
    side: Side,
    height: u8,
}

pub(crate) struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn init_game(&self) -> FieldResult<GameStateResult> {
        let db_game = init_game_state().await?;;
        let serialized = db_game.state;
        let game = State::deserialize(GameStateSerialized(serialized))?;
        Ok(GameStateResult::from_game(GameToken(db_game.id.to_string()), game))
    }
    async fn claim_player(&self, game_token: String, player: Player) -> Result<String, String> {

        Ok("todo".to_string())
    }
    async fn turn(&self, player_token: String, turn: Turn) -> Result<GameStateResult, String> {
        let db_game = fetch_game_state(&PlayerToken(player_token)).await?;
        let serialized = db_game.state;
        let game = State::deserialize(GameStateSerialized(serialized))?;
        Ok(GameStateResult::from_game(GameToken(db_game.id.to_string()), game))
    }
}