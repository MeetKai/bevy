use bevy_math::Vec3;
use std::sync::{Arc, Mutex};

use crate::{
    conversion::{XrFrom, XrInto},
    interaction::utils::*,
};

use web_sys::{XrFrame, XrPose, XrReferenceSpace, XrReferenceSpaceType};

pub struct TrackingSource {
    space_type: XrReferenceSpaceType,
    space: Arc<Mutex<XrReferenceSpace>>,
    frame: XrFrame,
}

impl TrackingSource {
    pub fn new(space: XrReferenceSpace, space_type: XrReferenceSpaceType, frame: XrFrame) -> Self {
        Self {
            space: Arc::new(Mutex::new(space)),
            space_type,
            frame,
        }
    }
}

unsafe impl Send for TrackingSource {}
unsafe impl Sync for TrackingSource {}

impl bevy_xr::interaction::implementation::XrTrackingSourceBackend for TrackingSource {
    fn reference_space_type(&self) -> bevy_xr::XrReferenceSpaceType {
        self.space_type.xr_into()
    }

    fn set_reference_space_type(
        &self,
        _reference_space_type: bevy_xr::XrReferenceSpaceType,
    ) -> bool {
        // we can't set a diferent reference_space_type at runtime
        // because WebXr uses a Promise to do that and Bevy doesn't have async capabilities.
        // We can only set this before the App initialization at main function.
        //@TODO: fix when it's possible to use async code inside bevy
        false
    }

    fn bounds_geometry(&self) -> Option<Vec<Vec3>> {
        let space = self.space.clone();
        let space = space.lock().unwrap();
        get_bounds_geometry(&space)
    }

    fn views_poses(&self) -> Vec<bevy_xr::XrPose> {
        let base_space = self.space.clone();
        let base_space = base_space.lock().unwrap();
        get_views_poses(&self.frame, &base_space)
    }

    fn hands_pose(&self) -> [Option<bevy_xr::XrPose>; 2] {
        let left_input_src = self.frame.session().input_sources().get(0);
        let right_input_src = self.frame.session().input_sources().get(1);

        let base_space = self.space.clone();
        let base_space = base_space.lock().unwrap();

        return [
            get_hands_pose(left_input_src, &self.frame, &base_space),
            get_hands_pose(right_input_src, &self.frame, &base_space),
        ];
    }

    fn hands_skeleton_pose(&self) -> [Option<Vec<bevy_xr::XrJointPose>>; 2] {
        let left_input_src = self.frame.session().input_sources().get(0);
        let right_input_src = self.frame.session().input_sources().get(1);

        let base_space = self.space.clone();
        let base_space = base_space.lock().unwrap();

        return [
            get_hands_skeleton_pose(left_input_src, &self.frame, &base_space),
            get_hands_skeleton_pose(right_input_src, &self.frame, &base_space),
        ];
    }

    fn hands_target_ray(&self) -> [Option<bevy_xr::XrPose>; 2] {
        let left_input_src = self.frame.session().input_sources().get(0);
        let right_input_src = self.frame.session().input_sources().get(1);

        let base_space = self.space.clone();
        let base_space = base_space.lock().unwrap();

        return [
            get_target_ray(left_input_src, &self.frame, &base_space),
            get_target_ray(right_input_src, &self.frame, &base_space),
        ];
    }

    fn viewer_target_ray(&self) -> bevy_xr::XrPose {
        let base_space = self.space.clone();
        let base_space = base_space.lock().unwrap();

        let viewer_pose = self.frame.get_viewer_pose(&base_space).unwrap();

        XrFrom::<XrPose>::xr_from(XrPose::from(viewer_pose))
    }
}
