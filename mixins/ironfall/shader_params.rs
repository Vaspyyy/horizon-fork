// Each frame: where the blasts are, for the pack's cloud and dust ring.
fn ironfall_shader_params(game: &crate::game::Game<'_>, partial: f32, params: &mut [[f32; 4]; 8]) {
    crate::game::ironfall_client::shader_params(game, partial, params);
}
