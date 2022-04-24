table! {
    use crate::adversary::BotIdMapping;
    use diesel::sql_types::{Nullable, Text, Uuid};
    games {
        id -> Uuid,
        state -> Text,
        player_red -> Nullable<Uuid>,
        player_blue -> Nullable<Uuid>,
        bot_id -> Nullable<BotIdMapping>,
    }
}