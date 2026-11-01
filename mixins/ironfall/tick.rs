// After a server level ticks: fly its shells and blow up the ones that land.
fn ironfall_level_tick(game: &mut crate::game::Game<'_>, dimension: &str) {
    crate::game::ironfall_glue::level_tick(game, dimension);
}
