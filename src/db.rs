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
use crate::adversary::BotId;
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

#[derive(Clone, Debug, DieselNewType, PartialEq, Eq, Hash)]
pub struct GameStateSerialized(pub String);

impl From<std::string::String> for GameStateSerialized {
    fn from(s: std::string::String) -> Self {
        GameStateSerialized(s)
    }
}

impl From<&str> for GameStateSerialized {
    fn from(s: &str) -> Self {
        GameStateSerialized(s.to_string())
    }
}

#[derive(Clone, Debug, NewType, DieselNewType, PartialEq, Eq, Hash)]
pub struct PlayerToken(pub Uuid);
#[derive(Clone, Debug, NewType, DieselNewType, PartialEq, Eq, Hash)]
pub struct GameToken(pub Uuid);

pub(crate) async fn init_game_state(bot: Option<BotId>) -> Result<DbGame, String> {
    use crate::db_schema_macro::games::dsl::*;
    let mut new_game = DbGame::new();
    new_game.bot_id = bot;
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
    use crate::db_schema_macro::games::dsl::*;
    let game = &games.filter(id.eq(game_token)).first::<DbGame>(&STATICS.db_connection.get().unwrap()).map_err(|e| e.to_string())?;
    Ok(game.clone())
}

pub(crate) async fn fetch_game_state_for_player(player_token: &PlayerToken) -> Result<DbGameAndPlayer, String> {
    use crate::db_schema_macro::games::dsl::*;
    let token = player_token.0;
    let game = &games.filter(player_red.eq(token).or(player_blue.eq(token))).first::<DbGame>(&STATICS.db_connection.get().unwrap()).map_err(|e| e.to_string())?;
    // warn: non exhaustive
    let player = if game.player_red == Some(player_token.clone()) {
        Player::Red
    } else {
        Player::Blue
    };
    Ok(DbGameAndPlayer { game: game.clone(), player })
}

pub(crate) async fn update_game_state(game_token: &GameToken, s: GameStateSerialized) -> Result<DbGame, String> {
    use crate::db_schema_macro::games::dsl::*;
    let mut game = fetch_game_state(game_token).await?;
    let conn: &PgConnection = &STATICS.db_connection.get().unwrap();
    game.state = s.clone();
    let r = diesel::update(&game)
        // .set(&game)
        .set(&game)
        .get_result::<DbGame>(conn).map_err(|e| e.to_string())?;
    SimpleBroker::publish(r.clone());
    Ok(r)
}

pub(crate) async fn claim_game_player(game_token: &GameToken, player: Player) -> Result<(Uuid, DbGame), String> {
    use crate::db_schema_macro::games::dsl::*;
    let conn: &PgConnection = &STATICS.db_connection.get().unwrap();
    let game = games.filter(id.eq(game_token)).first::<DbGame>(conn).map_err(|e| e.to_string())?.clone();
    if !game.can_player_join(&player) {
        return Err("game is full".to_string());
    }
    let new_id = Uuid::new_v4();
    let statement = match player {
        Player::Red => {
            if !game.player_red.is_none() {
                return Err("Player red already been claimed".to_string());
            }
            diesel::update(&game)
                .set(&DbGamePlayerRedUpdate {
                    id: game.id.clone(),
                    // TODO PlayerToken::new
                    player_red: PlayerToken(new_id),
                }).get_result::<DbGame>(conn)
        },
        Player::Blue => {
            if !game.player_blue.is_none() {
                return Err("Player blue already been claimed".to_string());
            }
            diesel::update(&game)
                .set(&DbGamePlayerBlueUpdate {
                    id: game.id.clone(),
                    player_blue: PlayerToken(new_id),
                }).get_result::<DbGame>(conn)

        },
    };
    let r = statement.map_err(|e| e.to_string())?;
    SimpleBroker::publish(r.clone());
    Ok((new_id, r))
}