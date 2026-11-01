// The commands Ironfall adds to the server's tree.
fn ironfall_register(commands: &mut Vec<crate::game::native_server::NativeCommand>) {
    crate::game::ironfall_glue::register(commands);
}
