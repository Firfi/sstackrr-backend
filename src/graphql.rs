use crate::db::fetch_game_state;
use crate::game::{Player, State};
use crate::game::GameSerializations;
use async_graphql::{EmptyMutation, EmptySubscription, FieldResult, Object, Schema};

pub(crate) struct QueryRoot;

#[Object]
impl QueryRoot {
    pub(crate) async fn game(&self, player_token: String) -> FieldResult<Vec<Vec<Option<Player>>>> {
        let serialized = fetch_game_state(&player_token).await?;
        let game = State::deserialize(serialized)?;
        Ok(game.to_rows())
    }
}

pub(crate) type GraphQlSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;