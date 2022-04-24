use uuid::Uuid;
use crate::adversary::BotId;
use crate::db::{GameStateSerialized, GameToken, PlayerToken};
use crate::db_schema_macro::games;
use crate::game::Player;

#[derive(Queryable, Insertable, Identifiable, AsChangeset, Clone)]
#[table_name="games"]
pub struct DbGame {
    pub id: GameToken,
    pub state: GameStateSerialized,
    pub player_red: Option<PlayerToken>,
    pub player_blue: Option<PlayerToken>,
    pub bot_id: Option<BotId>,
}

impl DbGame {
    // geez
    fn actor_count(&self) -> usize {
        let mut res: usize = 0;
        if self.player_red.is_some() {
            res += 1;
        }
        if self.player_blue.is_some() {
            res += 1;
        }
        if self.bot_id.is_some() {
            res += 1;
        }
        res
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.actor_count() >= 3 {
            return Err("Too many players".to_string());
        }
        Ok(())
    }
    pub fn can_player_join(&self, player: &Player) -> bool {
        self.actor_count() <= 1 && match player {
            Player::Red => self.player_red.is_none(),
            Player::Blue => self.player_blue.is_none(),
        }
    }
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

pub const DEFAULT_GAME_SIZE: u8 = 7;

pub fn empty_state(width: u8, height: u8) -> String {
    let mut res = String::new();
    for _ in 0..height {
        for _ in 0..width {
            res.push_str("0 ");
        }
        res.push_str("\n");
    }
    res
}

impl DbGame {
    pub fn new(width: u8, height: u8) -> DbGame {
        DbGame {
            id: GameToken(Uuid::new_v4()),
            state: empty_state(width, height).trim().into(),
            player_red: None,
            player_blue: None,
            bot_id: None,
        }
    }
}