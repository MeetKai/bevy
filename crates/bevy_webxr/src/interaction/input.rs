use bevy_ecs::world::Mut;
use bevy_log::info;
use bevy_xr::XrActionSet;
use wasm_bindgen::JsValue;

pub fn handle_input(action_set: Mut<XrActionSet>, frame: web_sys::XrFrame) {
    // frame.session().input_sources()
    info!("Input is being handled");
    web_sys::console::log(&js_sys::Array::from(&JsValue::from_str(
        "Input is being handled",
    )));
}
