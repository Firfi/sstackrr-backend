use uuid::Uuid;
use crate::db::{GameToken, PlayerToken};

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
    pub id: Uuid,
    pub state: String, // todo GameStateSerialized
    pub player_red: Option<Uuid>,
    pub player_blue: Option<Uuid>,
}

#[derive(Identifiable, AsChangeset, Clone)]
#[table_name="games"]
pub struct DbGamePlayerRedUpdate {
    pub id: Uuid,
    pub player_red: Uuid,
}

#[derive(Identifiable, AsChangeset, Clone)]
#[table_name="games"]
pub struct DbGamePlayerBlueUpdate {
    pub id: Uuid,
    pub player_blue: Uuid,
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
            id: Uuid::new_v4(), // TODO GameToken?
            state: EMPTY_STATE.trim().into(),
            player_red: None,
            player_blue: None,
        }
    }
}