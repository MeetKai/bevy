use bevy_utils::HashMap;
use serde::Deserialize;

pub const OCULUS_TOUCH_PROFILE: &'static str = include_str!("oculus-touch.json");

#[derive(Deserialize, Debug)]
pub struct WebXRProfile {
    #[serde(rename = "profileId")]
    pub profile_id: String,
    pub layouts: HashMap<String, Layout>,
    #[serde(rename = "fallbackProfileIds")]
    pub fallback_profile_ids: Vec<String>,
}
#[derive(Deserialize, Debug)]
pub struct Layout {
    #[serde(rename = "selectComponentId")]
    pub select_component_id: String,
    pub gamepad: WebXRGamepad,
    pub components: HashMap<String, WebXrComponent>,
}
#[derive(Deserialize, Debug)]
pub struct WebXrComponent {
    #[serde(rename = "type")]
    pub component_type: WebXRComponentType,
    pub reserved: Option<bool>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum WebXRComponentType {
    #[serde(rename = "trigger")]
    Trigger,
    #[serde(rename = "squeeze")]
    Squeeze,
    #[serde(rename = "touchpad")]
    Touchpad,
    #[serde(rename = "thumbstick")]
    Thumbstick,
    #[serde(rename = "button")]
    Button,
}
#[derive(Deserialize, Debug)]
pub struct WebXRGamepad {
    pub mapping: String,
    pub buttons: Vec<Option<String>>,
    pub axes: Vec<Option<WebXRAxis>>,
}
#[derive(Deserialize, Debug)]
pub struct WebXRAxis {
    #[serde(rename = "componentId")]
    pub component_id: String,
    pub axis: String,
}
