use diesel::prelude::*;
use dotenv::dotenv;
use std::env;
use crate::db_schema::{EMPTY_STATE, DbGame, DbGamePlayerRedUpdate, DbGamePlayerBlueUpdate};
use lazy_static::lazy_static;
use diesel::{
    r2d2::{Pool, ConnectionManager},
    pg::PgConnection
};
use async_graphql::InputObject;
use uuid::Uuid;
use crate::game::Player;

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

impl From<std::string::String> for GameStateSerialized {
    fn from(s: std::string::String) -> Self {
        GameStateSerialized(s)
    }
}

pub struct PlayerToken(pub String);
pub struct GameToken(pub String);

pub(crate) async fn init_game_state() -> Result<DbGame, String> {
    use crate::db_schema::games::dsl::*;
    let new_game = DbGame::new();
    let conn: &PgConnection = &VALUES.db_connection.get().unwrap();
    let r = diesel::insert_into(games)
        .values(&new_game)
        .get_result::<DbGame>(conn).map_err(|e| e.to_string())?;
    Ok(r)
}

pub struct DbGameAndPlayer {
    pub game: DbGame,
    pub player: Player
}

pub(crate) async fn fetch_game_state(player_token: &PlayerToken) -> Result<DbGameAndPlayer, String> {
    use crate::db_schema::games::dsl::*;
    let token = Uuid::parse_str(&player_token.0).map_err(|e| e.to_string())?;
    let game = &games.filter(player_red.eq(token).or(player_blue.eq(token))).load::<DbGame>(&VALUES.db_connection.get().unwrap()).map_err(|e| e.to_string())?[0];
    // warn: non exhaustive
    let player = if game.player_red == Some(Uuid::parse_str(&player_token.0).unwrap()) {
        Player::Red
    } else {
        Player::Blue
    };
    Ok(DbGameAndPlayer { game: game.clone(), player })
}

pub(crate) async fn update_game_state(player_token: &PlayerToken, s: GameStateSerialized) -> Result<DbGame, String> {
    use crate::db_schema::games::dsl::*;
    let mut game = fetch_game_state(player_token).await?.game;
    let conn: &PgConnection = &VALUES.db_connection.get().unwrap();
    game.state = s.0.clone();
    let r = diesel::update(&game)
        // .set(&game)
        .set(&game)
        .get_result::<DbGame>(conn).map_err(|e| e.to_string())?;
    Ok(r)
}

pub(crate) async fn claim_game_player(game_token: &GameToken, player: Player) -> Result<(Uuid, DbGame), String> {
    use crate::db_schema::games::dsl::*;
    let conn: &PgConnection = &VALUES.db_connection.get().unwrap();
    let mut game = games.filter(id.eq(Uuid::parse_str(&game_token.0).unwrap())).load::<DbGame>(conn).map_err(|e| e.to_string())?[0].clone();
    let new_id = Uuid::new_v4();
    let statement = match player {
        Player::Red => {
            if !game.player_red.is_none() {
                return Err("Player red already been claimed".to_string());
            }
            diesel::update(&game)
                .set(&DbGamePlayerRedUpdate {
                    id: game.id,
                    player_red: new_id,
                }).get_result::<DbGame>(conn)
        },
        Player::Blue => {
            if !game.player_blue.is_none() {
                return Err("Player blue already been claimed".to_string());
            }
            diesel::update(&game)
                .set(&DbGamePlayerBlueUpdate {
                    id: game.id,
                    player_blue: new_id,
                }).get_result::<DbGame>(conn)

        },
    };
    let r = statement.map_err(|e| e.to_string())?;
    Ok((new_id, r))
}