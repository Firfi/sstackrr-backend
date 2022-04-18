use uuid::Uuid;
use crate::db::{GameStateSerialized, GameToken, PlayerToken};

table! {
    games {
        id -> Uuid,
        state -> Text,
        player_red -> Nullable<Uuid>,
        player_blue -> Nullable<Uuid>,
    }
}

#[derive(Queryable, Insertable, Identifiable, AsChangeset, Clone)]
#[table_name="games"]
pub struct DbGame {
    pub id: GameToken,
    pub state: GameStateSerialized,
    pub player_red: Option<PlayerToken>,
    pub player_blue: Option<PlayerToken>,
}

#[derive(Identifiable, AsChangeset, Clone)]
#[table_name="games"]
pub struct DbGamePlayerRedUpdate {
    pub id: GameToken,
    pub player_red: PlayerToken,
}

#[derive(Identifiable, AsChangeset, Clone)]
#[table_name="games"]
pub struct DbGamePlayerBlueUpdate {
    pub id: GameToken,
    pub player_blue: PlayerToken,
}

// default side is 7, and it's a square
pub const EMPTY_STATE: &str = r#"
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
    "#;

impl DbGame {
    pub fn new() -> DbGame {
        DbGame {
            id: GameToken(Uuid::new_v4()),
            state: EMPTY_STATE.trim().into(),
            player_red: None,
            player_blue: None,
        }
    }
}