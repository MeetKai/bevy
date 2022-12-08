use bevy_ecs::world::Mut;
use bevy_log::info;
use bevy_xr::XrActionSet;
use wasm_bindgen::JsValue;

pub fn handle_input(action_set: Mut<XrActionSet>, frame: web_sys::XrFrame) {
    // frame.session().input_sources()
    // info!("Input is being handled");
    // let test = frame.session().input_sources();
    // let test2 = test;
    // for index in 0..test.length() {
    //     let source = test.get(index).unwrap();
    //     source.profiles()
    //     source.
    // }
}
