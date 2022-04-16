table! {
    games (id) {
        id -> Uuid,
        player_red -> Nullable<Uuid>,
        player_blue -> Nullable<Uuid>,
        state -> Text,
    }
}
