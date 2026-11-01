// A message from the server: the client half hears of impacts.
fn ironfall_from_server(game: &mut crate::game::Game<'_>, channel: &str, data: &[u8]) {
    crate::game::ironfall_client::from_server(game, channel, data);
}
