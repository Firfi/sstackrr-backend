table! {
    use crate::adversary::BotIdMapping;
    use diesel::types::{Uuid, Text, Nullable};
    games {
        id -> Uuid,
        state -> Text,
        player_red -> Nullable<Uuid>,
        player_blue -> Nullable<Uuid>,
        bot_id -> Nullable<BotIdMapping>,
    }
}