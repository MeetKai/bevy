use bevy_ecs::world::World;
use bevy_render::{
    camera::{ActiveCamera, Camera2d, Camera3d},
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotValue},
    renderer::RenderContext,
};

use crate::{CameraLeftEye, CameraRightEye};

pub struct MainPassDriverNode;

impl Node for MainPassDriverNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if let Some(camera_3d) = world.resource::<ActiveCamera<Camera3d>>().get() {
            graph.run_sub_graph(
                crate::draw_3d_graph::NAME,
                vec![SlotValue::Entity(camera_3d)],
            )?;
        }

        if let Some(camera_2d) = world.resource::<ActiveCamera<Camera2d>>().get() {
            graph.run_sub_graph(
                crate::draw_2d_graph::NAME,
                vec![SlotValue::Entity(camera_2d)],
            )?;
        }

        let mut vr_eyes = vec![];
        if world.contains_resource::<ActiveCamera<CameraLeftEye>>() {
            vr_eyes.push(world.resource::<ActiveCamera<CameraLeftEye>>().get());
        }
        if world.contains_resource::<ActiveCamera<CameraRightEye>>() {
            vr_eyes.push(world.resource::<ActiveCamera<CameraRightEye>>().get());
        }

        for cam in vr_eyes {
            if let Some(camera_3d) = cam {
                graph.run_sub_graph(
                    crate::draw_3d_graph::NAME,
                    vec![SlotValue::Entity(camera_3d)],
                )?;
            }
        }

        Ok(())
    }
}
