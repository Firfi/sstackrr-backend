use diesel::prelude::*;
use std::env;
use async_graphql::NewType;
use crate::db_schema::{DbGame, DbGamePlayerRedUpdate, DbGamePlayerBlueUpdate};
use lazy_static::lazy_static;
use diesel::{
    r2d2::{Pool, ConnectionManager},
    pg::PgConnection
};
use uuid::Uuid;
use crate::broker::SimpleBroker;
use crate::game::Player;

type PgPool = Pool<ConnectionManager<PgConnection>>;

struct Statics {
    db_connection: PgPool,
}

lazy_static! {
    static ref STATICS: Statics = {
       Statics {
           db_connection: PgPool::builder()
               .max_size(8)
               .build(ConnectionManager::new(env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set")))
               .expect("failed to create db connection_pool")
       }
    };
}

embed_migrations!();

pub fn run_embed_migrations() {
    embedded_migrations::run(&STATICS.db_connection.get().unwrap());
}

pub struct GameStateSerialized(pub String);

impl From<std::string::String> for GameStateSerialized {
    fn from(s: std::string::String) -> Self {
        GameStateSerialized(s)
    }
}

#[derive(Clone, Debug, NewType)]
pub struct PlayerToken(pub Uuid);
#[derive(Clone, Debug, NewType)]
pub struct GameToken(pub String);

pub(crate) async fn init_game_state() -> Result<DbGame, String> {
    use crate::db_schema::games::dsl::*;
    let new_game = DbGame::new();
    let conn: &PgConnection = &STATICS.db_connection.get().unwrap();
    let r = diesel::insert_into(games)
        .values(&new_game)
        .get_result::<DbGame>(conn).map_err(|e| e.to_string())?;
    Ok(r)
}

pub struct DbGameAndPlayer {
    pub game: DbGame,
    pub player: Player
}

pub(crate) async fn fetch_game_state(game_token: &GameToken) -> Result<DbGame, String> {
    use crate::db_schema::games::dsl::*;
    let token = Uuid::parse_str(&game_token.0).map_err(|e| e.to_string())?;
    let game = &games.filter(id.eq(token)).first::<DbGame>(&STATICS.db_connection.get().unwrap()).map_err(|e| e.to_string())?;
    Ok(game.clone())
}

pub(crate) async fn fetch_game_state_for_player(player_token: &PlayerToken) -> Result<DbGameAndPlayer, String> {
    use crate::db_schema::games::dsl::*;
    let token = player_token.0;
    let game = &games.filter(player_red.eq(token).or(player_blue.eq(token))).first::<DbGame>(&STATICS.db_connection.get().unwrap()).map_err(|e| e.to_string())?;
    // warn: non exhaustive
    let player = if game.player_red == Some(player_token.0) {
        Player::Red
    } else {
        Player::Blue
    };
    Ok(DbGameAndPlayer { game: game.clone(), player })
}

pub(crate) async fn update_game_state(player_token: &PlayerToken, s: GameStateSerialized) -> Result<DbGame, String> {
    use crate::db_schema::games::dsl::*;
    let mut game = fetch_game_state_for_player(player_token).await?.game;
    let conn: &PgConnection = &STATICS.db_connection.get().unwrap();
    game.state = s.0.clone();
    let r = diesel::update(&game)
        // .set(&game)
        .set(&game)
        .get_result::<DbGame>(conn).map_err(|e| e.to_string())?;
    SimpleBroker::publish(r.clone());
    Ok(r)
}

pub(crate) async fn claim_game_player(game_token: &GameToken, player: Player) -> Result<(Uuid, DbGame), String> {
    use crate::db_schema::games::dsl::*;
    let conn: &PgConnection = &STATICS.db_connection.get().unwrap();
    let game = games.filter(id.eq(Uuid::parse_str(&game_token.0).unwrap())).first::<DbGame>(conn).map_err(|e| e.to_string())?.clone();
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
    SimpleBroker::publish(r.clone());
    Ok((new_id, r))
}