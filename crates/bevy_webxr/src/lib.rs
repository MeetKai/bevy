use crate::{conversion::XrInto, util::fov_from_mat4};
use bevy_core_pipeline::{
    clear_color::ClearColorConfig, core_3d::Camera3d,
    tonemapping::Tonemapping,
};
use bevy_hierarchy::BuildChildren;
pub mod conversion;
pub mod initialization;
pub mod interaction;
pub mod util;
pub mod webxr_context;

use bevy_app::{App, Plugin};
use bevy_ecs::{
    prelude::*,
    system::{Commands, NonSend, Query, Res, ResMut, Resource},
};
use bevy_math::{Mat4, UVec2};
use bevy_reflect::prelude::*;
use bevy_render::{
    camera::{
        Camera, CameraProjection, CameraProjectionPlugin, CameraRenderGraph, ManualTextureViews,
        RenderTarget, Viewport,
    },
    primitives::Frustum,
    renderer::{RenderAdapterInfo, RenderDevice, RenderQueue},
    view::VisibleEntities,
};
use bevy_transform::prelude::{GlobalTransform, Transform, TransformBundle};
use bevy_utils::{default, Uuid};
use bevy_xr::{XrActionSet, XrSessionMode, XrSystem};
use initialization::InitializedState;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{XrWebGlLayer, XrView, XrEye, XrFrame, XrWebGlLayerInit, XrRenderStateInit, WebGlFramebuffer};
use webxr_context::*;

use crate::interaction::input::{handle_input, setup_interaction};

#[derive(Default)]
pub struct WebXrPlugin;

impl Plugin for WebXrPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugin(CameraProjectionPlugin::<WebXrPerspectiveProjection>::default());
        app.set_runner(webxr_runner);
        setup(&mut app.world);
        app.add_startup_system(setup_webxr_pawn);
        app.add_system(sync_head_tf);
        app.add_system(sync_frustum);
        app.add_system(update_manual_texture_views);
    }
}

fn sync_head_tf(
    mut head_tf_q: Query<&mut Transform, With<HeadMarker>>,
    xr_ctx: NonSend<WebXrContext>,
    frame: NonSend<web_sys::XrFrame>,
) {
    let reference_space = &xr_ctx.space_info.0;
    let viewer_pose = frame.get_viewer_pose(&reference_space).unwrap();
    let head_tf = viewer_pose.transform().xr_into();

    for mut tf in &mut head_tf_q {
        *tf = head_tf;
    }
}

fn sync_frustum(
    xr_ctx: NonSend<WebXrContext>,
    frame: NonSend<XrFrame>,
    mut left_q: Query<&mut Frustum, (With<LeftEyeMarker>, Without<RightEyeMarker>)>,
    mut right_q: Query<&mut Frustum, (With<RightEyeMarker>, Without<LeftEyeMarker>)>,
) {
    let reference_space = &xr_ctx.space_info.0;
    let viewer_pose = frame.get_viewer_pose(&reference_space).unwrap();

    let mut left_frustum = left_q.single_mut();
    let mut right_frustum = right_q.single_mut();

    let views: Vec<XrView> = viewer_pose
        .views()
        .iter()
        .map(|view| view.unchecked_into::<XrView>())
        .collect();

    let left_eye: &XrView = views
        .iter()
        .find(|view| view.eye() == XrEye::Left)
        .unwrap();

    *left_frustum = frustum_from_view(left_eye);

    let right_eye: &XrView = views
        .iter()
        .find(|view| view.eye() == XrEye::Right)
        .unwrap();

    *right_frustum = frustum_from_view(right_eye);
}

/// Copied from Bevy's `PerspectiveProjection`
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct WebXrPerspectiveProjection {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl CameraProjection for WebXrPerspectiveProjection {
    fn get_projection_matrix(&self) -> Mat4 {
        let mut mat = Mat4::perspective_infinite_reverse_rh(self.fov, self.aspect_ratio, self.near);
        mat.y_axis.y = -mat.y_axis.y;
        mat.y_axis.w = -mat.y_axis.w;
        mat
    }

    fn update(&mut self, width: f32, height: f32) {
        self.aspect_ratio = width / height;
    }

    fn far(&self) -> f32 {
        self.far
    }
}

impl Default for WebXrPerspectiveProjection {
    fn default() -> Self {
        WebXrPerspectiveProjection {
            fov: std::f32::consts::PI / 2.0,
            aspect_ratio: 1.0,
            near: 0.01,
            far: 1000.0,
        }
    }
}
#[derive(Bundle)]
pub struct WebXrCamera3dBundle {
    pub camera: Camera,
    pub camera_render_graph: CameraRenderGraph,
    pub projection: WebXrPerspectiveProjection,
    pub visible_entities: VisibleEntities,
    pub frustum: Frustum,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub camera_3d: Camera3d,
    pub tonemapping: Tonemapping,
}

impl Default for WebXrCamera3dBundle {
    fn default() -> Self {
        Self {
            camera_render_graph: CameraRenderGraph::new(bevy_core_pipeline::core_3d::graph::NAME),
            tonemapping: Tonemapping::Enabled {
                deband_dither: true,
            },
            camera: Default::default(),
            projection: Default::default(),
            visible_entities: Default::default(),
            frustum: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            camera_3d: Default::default(),
        }
    }
}

//Get (incomplete) bevy camera bundle from web_sys Mat4
fn get_camera_3d_bundle_from_webxr_proj_mat(mat: Mat4) -> WebXrCamera3dBundle {
    let fov = fov_from_mat4(mat);
    let tf = Transform::IDENTITY;

    // TODO: actual near/far plane conversion
    let projection = WebXrPerspectiveProjection {
        fov,
        far: 1000.0,
        ..default()
    };
    let view_proj = projection.get_projection_matrix() * tf.compute_matrix().inverse();
    let frustum =
        Frustum::from_view_projection(&view_proj, &tf.translation, &tf.back(), projection.far);

    WebXrCamera3dBundle {
        projection,
        frustum,
        ..default()
    }
}

pub fn frustum_from_view(view: &XrView) -> Frustum {
    let proj: Mat4 = view.projection_matrix().xr_into();

    let fov = fov_from_mat4(proj);

    let tf: Transform = view.transform().xr_into();

    // TODO: actual near/far plane conversion
    let projection = WebXrPerspectiveProjection {
        fov,
        far: 1000.0,
        ..default()
    };

    let view_proj = projection.get_projection_matrix() * tf.compute_matrix().inverse();

    Frustum::from_view_projection(&view_proj, &tf.translation, &tf.back(), projection.far)
}

/// Sets up mid-level webxr pawn,
/// Spawns a Head Entity with two eyes as Entities
fn setup_webxr_pawn(
    xr_ctx: NonSend<WebXrContext>,
    frame: NonSend<XrFrame>,
    id: Res<FramebufferUuid>,
    mut commands: Commands,
) {
    let reference_space = &xr_ctx.space_info.0;
    let viewer_pose = frame.get_viewer_pose(&reference_space).unwrap();

    let head_tf: Transform = viewer_pose.transform().xr_into();

    let views: Vec<XrView> = viewer_pose
        .views()
        .iter()
        .map(|view| view.unchecked_into::<XrView>())
        .collect();

    let left_eye: &XrView = views
        .iter()
        .find(|view| view.eye() == XrEye::Left)
        .unwrap();

    let left_proj: Mat4 = left_eye.projection_matrix().xr_into();

    let viewport_id = id.0;

    let base_layer: XrWebGlLayer = frame.session().render_state().base_layer().unwrap();

    let resolution = UVec2::new(
        base_layer.framebuffer_width(),
        base_layer.framebuffer_height(),
    );
    let physical_size = UVec2::new(resolution.x / 2, resolution.y);

    let left_viewport = Viewport {
        physical_position: UVec2::ZERO,
        physical_size,
        ..default()
    };

    let right_viewport = Viewport {
        physical_position: UVec2::new(resolution.x / 2, 0),
        physical_size,
        ..default()
    };

    commands
        .spawn((
            TransformBundle {
                local: head_tf,
                ..default()
            },
            HeadMarker,
        ))
        .with_children(|head| {
            head.spawn((
                WebXrCamera3dBundle {
                    camera: Camera {
                        target: RenderTarget::TextureView(viewport_id),
                        viewport: Some(left_viewport),
                        ..default()
                    },
                    ..get_camera_3d_bundle_from_webxr_proj_mat(left_proj)
                },
                LeftEyeMarker,
            ));
            head.spawn((
                WebXrCamera3dBundle {
                    camera: Camera {
                        target: RenderTarget::TextureView(viewport_id),
                        priority: 1,
                        viewport: Some(right_viewport),
                        ..default()
                    },
                    camera_3d: Camera3d {
                        //Viewport does not affect ClearColor, so we set the right camera to a None Clear Color
                        clear_color: ClearColorConfig::None,
                        ..default()
                    },
                    ..get_camera_3d_bundle_from_webxr_proj_mat(left_proj)
                },
                RightEyeMarker,
            ));
        });
}

/// Resource that contains the `Uuid` corresponding to WebGlFramebuffer
#[derive(Resource)]
pub struct FramebufferUuid(pub Uuid);

#[derive(Resource)]
/// Wrapper for the WebXR Framebuffer
pub struct VrFramebufferTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl VrFramebufferTexture {
    pub fn new(texture: wgpu::Texture) -> Self {
        Self {
            view: texture.create_view(&Default::default()),
            texture,
        }
    }

    pub fn new_cubemap(texture: wgpu::Texture) -> Self {
        Self {
            view: texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..Default::default()
            }),
            texture,
        }
    }
}

fn setup(world: &mut World) {
    let InitializedState {
        webgl2_context,
        webxr_context,
        adapter,
        device,
        queue,
    } = world.remove_resource().unwrap();
    let adapter_info = adapter.get_info();
    let layer_init = XrWebGlLayerInit::new();

    let xr_gl_layer = XrWebGlLayer::new_with_web_gl2_rendering_context_and_layer_init(
        &webxr_context.session,
        &webgl2_context,
        &layer_init,
    )
    .unwrap();

    let mut render_state_init = XrRenderStateInit::new();
    render_state_init
        .depth_near(0.001)
        .base_layer(Some(&xr_gl_layer));

    webxr_context
        .session
        .update_render_state_with_state(&render_state_init);

    world.init_resource::<XrActionSet>();
    world.insert_resource(RenderDevice::from(Arc::new(device)));
    world.insert_resource(RenderQueue(Arc::new(queue)));
    world.insert_resource(RenderAdapterInfo(adapter_info));
    world.insert_non_send_resource(webxr_context);
    let id = Uuid::new_v4();
    world.insert_resource(FramebufferUuid(id));
}

/// System that updates `ManualTextureViews` with the new `WebGlFramebuffer`
fn update_manual_texture_views(
    frame: bevy_ecs::prelude::NonSend<XrFrame>,
    device: Res<RenderDevice>,
    framebuffer_uuid: Res<FramebufferUuid>,
    mut manual_tex_view: ResMut<ManualTextureViews>,
) {
    let base_layer: XrWebGlLayer = frame.session().render_state().base_layer().unwrap();

    //Reflect hack because base_layer.framebuffer is technically null
    let framebuffer: WebGlFramebuffer =
        js_sys::Reflect::get(&base_layer, &"framebuffer".into())
            .unwrap()
            .into();
    let device = device.wgpu_device();

    let framebuffer_colour_attachment: VrFramebufferTexture = create_view_from_device_framebuffer(
        device,
        framebuffer.clone(),
        &base_layer,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        "Device Framebuffer (Color)",
    );

    let resolution = UVec2::new(
        base_layer.framebuffer_width(),
        base_layer.framebuffer_height(),
    );

    manual_tex_view.insert(
        framebuffer_uuid.0,
        (framebuffer_colour_attachment.view.into(), resolution),
    );
}

/// Bevy runner that works with WebXR
fn webxr_runner(mut app: App) {
    let webxr_context = app.world.get_non_send_resource::<WebXrContext>().unwrap();
    let session = webxr_context.session.clone();
    type XrFrameHandler = Closure<dyn FnMut(f64, XrFrame)>;
    let f: Rc<RefCell<Option<XrFrameHandler>>> = Rc::new(RefCell::new(None));
    let g: Rc<RefCell<Option<XrFrameHandler>>> = f.clone();

    // TODO: Update with accurate availble session modes when async is supported
    app.world
        .insert_resource(XrSystem::new(vec![XrSessionMode::ImmersiveVR]));
    println!("inserted XrSystem");

    *g.borrow_mut() = Some(Closure::new(move |_time: f64, frame: XrFrame| {
        setup_interaction(&frame, &mut app.world);
        let action_set = &mut app.world.get_resource_mut::<XrActionSet>().unwrap();
        handle_input(action_set, &frame);
        app.world.insert_non_send_resource(frame.clone());

        app.update();

        let session = frame.session();
        session.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }));
    session.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref());
}

#[cfg(target_arch = "wasm32")]
pub fn create_view_from_device_framebuffer(
    device: &wgpu::Device,
    framebuffer: WebGlFramebuffer,
    base_layer: &XrWebGlLayer,
    format: wgpu::TextureFormat,
    label: &'static str,
) -> VrFramebufferTexture {
    VrFramebufferTexture::new(unsafe {
        device.create_texture_from_hal::<wgpu_hal::api::Gles>(
            wgpu_hal::gles::Texture {
                inner: wgpu_hal::gles::TextureInner::ExternalFramebuffer { inner: framebuffer },
                mip_level_count: 1,
                array_layer_count: 1,
                format,
                format_desc: wgpu_hal::gles::TextureFormatDesc {
                    internal: glow::RGBA,
                    external: glow::RGBA,
                    data_type: glow::UNSIGNED_BYTE,
                },
                copy_size: wgpu_hal::CopyExtent {
                    width: base_layer.framebuffer_width(),
                    height: base_layer.framebuffer_height(),
                    depth: 1,
                },
                is_cubemap: false,
                drop_guard: None,
            },
            &wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: base_layer.framebuffer_width(),
                    height: base_layer.framebuffer_height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            },
        )
    })
}

#[derive(Component, Debug)]
pub struct HeadMarker;

#[derive(Component)]
pub struct LeftEyeMarker;

#[derive(Component)]
pub struct RightEyeMarker;
