use slab::Slab;

pub struct Game {
    state: String,
}



pub(crate) async fn fetch_game_state(user_token: &String) -> Result<String, String> {
    // const test_token: String = "test_token".into();
    let game: Box<Game> = Box::new(Game {
        state: r#"
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
0 0 0 0 0 0 0
    "#.into()
    });
    Ok(game.state.clone())
}

pub(crate) async fn update_game_state(user_token: &String, state: String) -> Result<(), String> {
    // GAME.state = state;
    Ok(())
}