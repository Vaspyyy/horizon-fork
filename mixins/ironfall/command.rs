// A player's command on the server: Ironfall's, when it is one.
fn ironfall_perform_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    if !*ran {
        crate::game::ironfall_glue::perform(game, command, ran);
    }
}
