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

#[derive(Queryable, Insertable, Clone)]
#[table_name="games"]
pub struct Game {
    pub id: Uuid,
    pub state: String, // todo GameStateSerialized
    pub player_red: Option<Uuid>,
    pub player_blue: Option<Uuid>,
}

pub const EMPTY_STATE: &str = r#"
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
    "#;

impl Game {
    pub fn new() -> Game {
        Game {
            id: Uuid::new_v4(), // TODO GameToken?
            state: EMPTY_STATE.trim().into(),
            player_red: None,
            player_blue: None,
        }
    }
}