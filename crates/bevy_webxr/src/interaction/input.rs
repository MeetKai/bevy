use bevy_ecs::world::Mut;
use bevy_xr::XrActionSet;

pub fn handle_input(action_set: Mut<XrActionSet>, frame: web_sys::XrFrame) {
    // frame.session().input_sources()
    println!("Input is being handled");
}
