use std::collections::HashMap;

use bevy_ecs::world::{Mut, World};

use bevy_log::{info, warn};
use bevy_math::Vec2;
use bevy_xr::{
    XrActionDescriptor, XrActionSet, XrActionState, XrActionType, XrButtonState,
    XrProfileDescriptor, XrSessionMode, XrSystem,
};

use wasm_bindgen::JsValue;
use web_sys::XrHandedness;

use crate::{conversion::XrInto, interaction::profiles::OCULUS_TOUCH_PROFILE};

use super::profiles::{WebXRComponentType, WebXRProfile};

pub fn setup_interaction(frame: &web_sys::XrFrame, world: &mut World) {
    let mut xr_system = match world.get_resource_mut::<XrSystem>() {
        Some(r) => r,
        None => {
            world.insert_resource(XrSystem::new(vec![XrSessionMode::ImmersiveVR]));
            world.get_resource_mut::<XrSystem>().unwrap()
        }
    };

    if xr_system.action_set().len() == 0 && frame.session().input_sources().length() > 0 {
        let mut bindings = Vec::new();
        for index in 0..frame.session().input_sources().length() {
            let input_source = frame.session().input_sources().get(index).unwrap();
            if input_source
                .profiles()
                .to_vec()
                .contains(&JsValue::from_str("oculus-touch"))
            {
                bindings.append(&mut create_oculus_bindings(input_source.handedness()))
            }
        }
        xr_system.set_action_set(vec![XrProfileDescriptor {
            profile: "oculus-touch".into(),
            bindings,
            tracked: true,
            has_haptics: true,
        }]);
    }
}

pub fn handle_input(action_set: &mut Mut<XrActionSet>, frame: &web_sys::XrFrame) {
    let oculus_profile = create_oculus_profile();
    let mut states = HashMap::new();

    for index in 0..frame.session().input_sources().length() {
        let input_source = frame.session().input_sources().get(index).unwrap();
        let handedness_string: String = input_source.handedness().xr_into();

        let (layout, gamepad) = match (
            oculus_profile.layouts.get(&handedness_string),
            input_source.gamepad(),
        ) {
            (Some(l), Some(g)) => (l, g),
            _ => {
                warn!("layout or gamepad does not exist");
                continue;
            }
        };

        // Poll buttons
        for (index, button) in layout.gamepad.buttons.iter().enumerate() {
            let button = match button {
                Some(b) => b,
                None => continue,
            };

            let js_button = gamepad.buttons().get(index as u32);
            use js_sys::Reflect;
            let value = Reflect::get(&js_button, &JsValue::from_str("value"))
                .map(|v| v.as_f64())
                .map(|x| x.map(|x| x as f32))
                .unwrap_or(None);

            let pressed = Reflect::get(&js_button, &JsValue::from_str("pressed"))
                .map(|v| v.as_bool())
                .unwrap_or(None);

            let touched = Reflect::get(&js_button, &JsValue::from_str("touched"))
                .map(|v| v.as_bool())
                .unwrap_or(None);

            if button == "xr-standard-trigger" && handedness_string == "right" {
                info!("{:?}", value.clone());
            }

            let state = match (pressed, touched) {
                (Some(true), _) => XrButtonState::Pressed,
                (Some(false), Some(true)) => XrButtonState::Touched,
                (_, _) => XrButtonState::Default,
            };

            states.insert(
                format!("{}/{}", handedness_string, button),
                bevy_xr::XrActionState::Button {
                    state,
                    value: value.unwrap_or(-1.0),
                },
            );
        }

        // Poll axes
        let mut axes_values = HashMap::<String, Vec2>::new();
        for (index, axis) in layout.gamepad.axes.iter().enumerate() {
            let axis = match axis {
                Some(a) => a,
                None => continue,
            };

            let axis_value = match gamepad.axes().get(index as u32).as_f64() {
                Some(v) => v as f32,
                None => continue,
            };

            let value = axes_values
                .entry(format!("{}/{}", handedness_string, axis.component_id))
                .or_default();
            if axis.axis == "x-axis" {
                value.x = axis_value;
            } else {
                value.y = axis_value;
            }
        }

        for (name, value) in axes_values {
            states.insert(name, XrActionState::Vec2D(value));
        }
    }
    action_set.set(states)
}

fn create_oculus_profile() -> WebXRProfile {
    serde_json::from_str(OCULUS_TOUCH_PROFILE).expect("Error parsing")
}

fn create_oculus_bindings(handedness: XrHandedness) -> Vec<(XrActionDescriptor, String)> {
    if handedness == XrHandedness::None {
        panic!("Expected left or right handedness");
    }
    let handedness_string: String = handedness.xr_into();
    let profile = create_oculus_profile();
    let layout = match profile.layouts.get(&handedness_string) {
        Some(l) => l,
        None => return Vec::new(),
    };
    let mut output = Vec::new();
    for (key, component) in &layout.components {
        let action_type = match component.component_type {
            WebXRComponentType::Trigger => XrActionType::Button {
                touch: true,
                click: false,
                value: true,
            },
            WebXRComponentType::Squeeze => XrActionType::Button {
                touch: true,
                click: false,
                value: true,
            },
            WebXRComponentType::Touchpad => XrActionType::Vec2D,
            WebXRComponentType::Thumbstick => XrActionType::Vec2D,
            WebXRComponentType::Button => XrActionType::Button {
                touch: true,
                click: false,
                value: true,
            },
        };
        output.push((
            XrActionDescriptor {
                name: format!("{}/{}", handedness_string, key),
                action_type,
            },
            format!("{}/{}", handedness_string, key),
        ));
    }
    output
}
