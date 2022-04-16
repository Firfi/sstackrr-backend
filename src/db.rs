use diesel::prelude::*;
use dotenv::dotenv;
use std::env;
use crate::db_schema::{EMPTY_STATE, Game};
use lazy_static::lazy_static;
use diesel::{
    r2d2::{Pool, ConnectionManager},
    pg::PgConnection
};
use async_graphql::InputObject;
use uuid::Uuid;

type PgPool = Pool<ConnectionManager<PgConnection>>;

pub struct Values {
    pub db_connection: PgPool,
}

lazy_static! {
    pub static ref VALUES: Values = {
       Values {
           db_connection: PgPool::builder()
               .max_size(8)
               .build(ConnectionManager::new(env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set")))
               .expect("failed to create db connection_pool")
       }
    };
}

pub struct GameStateSerialized(pub String);

pub struct PlayerToken(pub String);
pub struct GameToken(pub String);

pub(crate) async fn init_game_state() -> Result<Game, String> {
    use crate::db_schema::games::dsl::*;
    let new_game = Game::new();
    let conn: &PgConnection = &VALUES.db_connection.get().unwrap();
    let r = diesel::insert_into(games)
        .values(&new_game)
        .get_result::<Game>(conn).map_err(|e| e.to_string())?;
    Ok(r)
}

pub(crate) async fn fetch_game_state(player_token: &PlayerToken) -> Result<Game, String> {
    use crate::db_schema::games::dsl::*;
    let token = Uuid::parse_str(&player_token.0).map_err(|e| e.to_string())?;
    let results = games.filter(player_red.eq(token).or(player_blue.eq(token))).load::<Game>(&VALUES.db_connection.get().unwrap()).map_err(|e| e.to_string())?;
    Ok(results[0].clone())
}

pub(crate) async fn update_game_state(player_token: &PlayerToken, state: GameStateSerialized) -> Result<(), String> {
    // GAME.state = state;
    Ok(())
}