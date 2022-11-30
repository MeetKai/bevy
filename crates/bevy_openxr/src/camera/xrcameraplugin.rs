use bevy_app::{App, CoreStage, Plugin};

use bevy_core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transparent3d};
use bevy_ecs::{
    prelude::{Component, World},
    schedule::IntoSystemDescriptor,
    system::{Commands, Res},
};
use bevy_render::{
    camera::{camera_system, CameraProjectionPlugin},
    render_graph::{self, NodeRunError, RenderGraph, RenderGraphContext, SlotValue},
    render_phase::RenderPhase,
    renderer::RenderContext,
    view::{update_frusta, VisibilitySystems},
    RenderApp, RenderStage,
};
use bevy_transform::TransformSystem;
use bevy_window::ModifiesWindows;

use super::{update_xrcamera_view, XRProjection};

#[derive(Component, Default)]
pub struct XrCameraLeftMarker;
#[derive(Component, Default)]
pub struct XrCameraRightMarker;

#[derive(Default)]
pub struct XrCameraPlugin;
impl Plugin for XrCameraPlugin {
    fn build(&self, app: &mut App) {
        // app.add_plugin(CameraTypePlugin::<XrCameraLeftMarker>::default());
        // app.add_plugin(CameraTypePlugin::<XrCameraRightMarker>::default());

        let render_app = app.sub_app_mut(RenderApp);

        // // add `RenderPhase<Opaque3d>`, `RenderPhase<AlphaMask3d>` and `RenderPhase<Transparent3d>` camera phases
        // render_app.add_system_to_stage(RenderStage::Extract, extract_xr_camera_phases);

        // // add a render graph node that executes the 3d subgraph
        // let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        // let xr_camera_node = render_graph.add_node("xr_cameras", XrCameraDriverNode);
        // render_graph
        //     .add_node_edge(
        //         bevy_core_pipeline::node::MAIN_PASS_DEPENDENCIES,
        //         xr_camera_node,
        //     )
        //     .unwrap();
        // render_graph
        //     .add_node_edge(bevy_core_pipeline::node::CLEAR_PASS_DRIVER, xr_camera_node)
        //     .unwrap();

        //  make sure XRProjection-based cameras are processed
        app.add_plugin(CameraProjectionPlugin::<XRProjection>::default());

        app.add_system_to_stage(
            CoreStage::PostUpdate,
            update_frusta::<XRProjection>
                .after(TransformSystem::TransformPropagate)
                //  ensures we execute at the right time without adding more labels
                .before(VisibilitySystems::UpdatePerspectiveFrusta),
        );

        app.add_system_to_stage(CoreStage::PreUpdate, update_xrcamera_view);
    }
}

pub struct XrCameraDriverNode;

// impl render_graph::Node for XrCameraDriverNode {
//     fn run(
//         &self,
//         graph: &mut RenderGraphContext,
//         _: &mut RenderContext,
//         world: &World,
//     ) -> Result<(), NodeRunError> {
//         if let Some(camera) = world.resource::<ActiveCamera<XrCameraLeftMarker>>().get() {
//             graph.run_sub_graph(
//                 bevy_core_pipeline::draw_3d_graph::NAME,
//                 vec![SlotValue::Entity(camera)],
//             )?;
//         }

//         if let Some(camera) = world.resource::<ActiveCamera<XrCameraRightMarker>>().get() {
//             graph.run_sub_graph(
//                 bevy_core_pipeline::draw_3d_graph::NAME,
//                 vec![SlotValue::Entity(camera)],
//             )?;
//         }

//         Ok(())
//     }
// }

// pub fn extract_xr_camera_phases(
//     mut commands: Commands,
//     left: Res<ActiveCamera<XrCameraLeftMarker>>,
//     right: Res<ActiveCamera<XrCameraRightMarker>>,
// ) {
//     if let Some(entity) = left.get() {
//         commands.get_or_spawn(entity).insert_bundle((
//             RenderPhase::<Opaque3d>::default(),
//             RenderPhase::<AlphaMask3d>::default(),
//             RenderPhase::<Transparent3d>::default(),
//         ));
//     }
//     if let Some(entity) = right.get() {
//         commands.get_or_spawn(entity).insert_bundle((
//             RenderPhase::<Opaque3d>::default(),
//             RenderPhase::<AlphaMask3d>::default(),
//             RenderPhase::<Transparent3d>::default(),
//         ));
//     }
// }
