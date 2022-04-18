use crate::db::{claim_game_player, fetch_game_state_for_player, GameStateSerialized, GameToken, init_game_state, PlayerToken, update_game_state, fetch_game_state};
use crate::game::{GameOperations, Player, Side, State};
use crate::game::GameSerializations;
use async_graphql::{FieldResult, Object, SimpleObject, InputObject, Schema, Subscription};
use async_graphql::futures_util::Stream;
use tokio_stream::StreamExt;
use crate::broker::SimpleBroker;
use crate::db_schema::DbGame;

#[derive(SimpleObject)]
pub struct GameStateResult {
    id: GameToken,
    state: Vec<Vec<Option<Player>>>,
    next_player: Option<Player>,
    winner: Option<Player>,
    is_stalemate: bool,
    red_claimed: bool,
    blue_claimed: bool,
}

#[derive(SimpleObject)]
pub struct ClaimPlayerResult {
    game: GameStateResult,
    player_token: PlayerToken,
}

impl GameStateResult {
    pub fn from_db_game(db_game: &DbGame) -> GameStateResult {
        let game = game_from_db_game(db_game).unwrap();
        GameStateResult {
            id: db_game.id.clone(),
            state: game.to_rows(),
            next_player: if game.is_finished() || game.is_stalemate() { None } else { Some(game.next_player().unwrap()) },
            winner: game.try_winner(),
            is_stalemate: game.is_stalemate(),
            red_claimed: db_game.player_red.is_some(),
            blue_claimed: db_game.player_blue.is_some(),
        }
    }
}

pub(crate) struct QueryRoot;

fn game_from_db_game(db_game: &DbGame) -> Result<State, String> {
    State::deserialize(&db_game.state)
}

#[Object]
impl QueryRoot {
    pub(crate) async fn game(&self, game_token: GameToken) -> FieldResult<GameStateResult> {
        Ok(GameStateResult::from_db_game(&fetch_game_state(&game_token).await?))
    }
    pub(crate) async fn me(&self, player_token: PlayerToken) -> FieldResult<Player> {
        Ok(fetch_game_state_for_player(&player_token).await?.player)
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
        Ok(GameStateResult::from_db_game(&init_game_state().await?))
    }
    async fn claim_player(&self, game_token: GameToken, player: Player) -> Result<ClaimPlayerResult, String> {
        let (id, db_game) = claim_game_player(&game_token, player).await?;
        let game = GameStateResult::from_db_game(&db_game);
        Ok(ClaimPlayerResult {
            player_token: PlayerToken(id),
            game,
        })
    }
    async fn turn(&self, player_token: PlayerToken, turn: TurnInput) -> Result<GameStateResult, String> {
        let db_game_and_player = fetch_game_state_for_player(&player_token).await?;
        let db_game = db_game_and_player.game;
        let player = db_game_and_player.player;
        let mut state = game_from_db_game(&db_game)?;
        state.push((player, turn.height, turn.side))?;
        let new_db_game = update_game_state(&player_token, state.serialize()).await?;
        Ok(GameStateResult::from_db_game(&new_db_game))
    }
}

pub(crate) struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    // a "readonly" game for anyone to subscribe to. I push the whole game state, because I'm lazy and also it isn't big size anyways
    async fn game(&self, game_token: GameToken) -> impl Stream<Item = GameStateResult> {
        SimpleBroker::<DbGame>::subscribe().filter(move |db_game: &DbGame| {
            db_game.id == game_token
        }).map(|db_game: DbGame| {
            GameStateResult::from_db_game(&db_game)
        })
    }
}