// Each frame: the camera shake of blasts whose wave just arrived.
fn ironfall_camera_shake(
    game: &crate::game::Game<'_>,
    partial: f32,
    shake: &mut crate::game::native_hooks::NativeShake,
) {
    crate::game::ironfall_client::camera_shake(game, partial, shake);
}
