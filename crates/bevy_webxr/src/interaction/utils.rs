use bevy_math::Vec3;

use wasm_bindgen::JsValue;
use web_sys::{
    DomPointReadOnly, XrBoundedReferenceSpace, XrFrame, XrHand, XrInputSource, XrJointSpace,
    XrReferenceSpace, XrSpace, XrView, XrViewerPose,
};

use crate::conversion::XrInto;

pub fn js_value_to_vec3(value: JsValue) -> Vec3 {
    let point = DomPointReadOnly::from(value);
    Vec3::new(point.x() as f32, point.y() as f32, point.z() as f32)
}

pub fn js_value_to_pose(value: &JsValue, viewer_pose: &XrViewerPose) -> bevy_xr::XrPose {
    let view = XrView::from(value.clone());
    bevy_xr::XrPose {
        transform: view.transform().xr_into(),
        linear_velocity: None,
        angular_velocity: None,
        emulated_position: viewer_pose.emulated_position(),
    }
}

pub fn get_pose(
    frame: &XrFrame,
    space: &XrSpace,
    base_space: &XrReferenceSpace,
) -> bevy_xr::XrPose {
    frame.get_pose(space, &base_space).unwrap().xr_into()
}

pub fn get_values(src: &XrInputSource) -> js_sys::Array {
    js_sys::Object::values(<XrHand as AsRef<js_sys::Object>>::as_ref(
        &src.hand().unwrap(),
    ))
}

pub fn js_value_to_joint_pose(
    frame: &XrFrame,
    value: &JsValue,
    base_space: &XrReferenceSpace,
) -> bevy_xr::XrJointPose {
    let joint_space = XrJointSpace::from(value.clone());
    frame
        .get_joint_pose(&joint_space, &base_space)
        .unwrap()
        .xr_into()
}

pub fn get_hands_skeleton_pose(
    src: Option<XrInputSource>,
    frame: &XrFrame,
    base_space: &XrReferenceSpace,
) -> Option<Vec<bevy_xr::XrJointPose>> {
    src.map(|src| {
        get_values(&src)
            .iter()
            .map(|js_value| js_value_to_joint_pose(frame, &js_value, &base_space))
            .collect()
    })
}

pub fn get_target_ray(
    src: Option<XrInputSource>,
    frame: &XrFrame,
    base_space: &XrReferenceSpace,
) -> Option<bevy_xr::XrPose> {
    src.map(|src| get_pose(frame, &src.target_ray_space(), base_space))
}

pub fn get_hands_pose(
    src: Option<XrInputSource>,
    frame: &XrFrame,
    base_space: &XrReferenceSpace,
) -> Option<bevy_xr::XrPose> {
    src.map(|src| get_pose(frame, &src.grip_space().unwrap(), base_space))
}

pub fn get_views_poses(frame: &XrFrame, base_space: &XrReferenceSpace) -> Vec<bevy_xr::XrPose> {
    frame
        .get_viewer_pose(base_space)
        .map_or(vec![], |viewer_pose| {
            return viewer_pose
                .views()
                .iter()
                .map(|js_value| js_value_to_pose(&js_value, &viewer_pose))
                .collect();
        })
}

pub fn get_bounds_geometry(space: &XrReferenceSpace) -> Option<Vec<Vec3>> {
    let space =
        XrBoundedReferenceSpace::from(<XrReferenceSpace as AsRef<JsValue>>::as_ref(space).clone());
    Some(
        space
            .bounds_geometry()
            .iter()
            .map(|js_value| js_value_to_vec3(js_value.clone()))
            .collect(),
    )
}
