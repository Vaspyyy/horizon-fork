// Once a tick in every game: the client half counts ticks in the window's.
fn ironfall_client_tick(game: &crate::game::Game<'_>) {
    crate::game::ironfall_client::tick(game);
}
