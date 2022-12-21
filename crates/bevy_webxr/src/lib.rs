use crate::conversion::XrInto;
use bevy_core_pipeline::{
    clear_color::ClearColorConfig,
    core_3d::{Camera3d, Camera3dBundle},
};
use bevy_hierarchy::BuildChildren;
pub mod conversion;
pub mod initialization;
pub mod interaction;
pub mod webxr_context;

use bevy_app::{App, Plugin};
use bevy_asset::{AssetServer, Assets};
use bevy_ecs::{
    prelude::{Component, With, Without, World},
    system::{Commands, NonSend, Query, Res, ResMut, Resource},
};
use bevy_math::UVec2;
use bevy_pbr::{PbrBundle, StandardMaterial};
use bevy_render::prelude::*;
use bevy_render::{
    camera::{Camera, ManualTextureViews, RenderTarget, Viewport},
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    renderer::{RenderAdapterInfo, RenderDevice, RenderQueue},
    view::Visibility,
};
use bevy_transform::{prelude::Transform, TransformBundle};
use bevy_utils::{default, Uuid};
use initialization::InitializedState;
use interaction::TrackingSource;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::XrWebGlLayer;
use webxr_context::*;

#[derive(Default)]
pub struct WebXrPlugin;

impl Plugin for WebXrPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.set_runner(webxr_runner);
        setup(&mut app.world);
        app.add_startup_system(setup_webxr_pawn);
        app.add_system(sync_head_tf);
        app.add_system(sync_hands_tf);
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
    // bevy_log::info!("head transform before {:?}", viewer_pose.transform().position().y());
    let head_tf = viewer_pose.transform().xr_into();

    for mut tf in &mut head_tf_q {
        bevy_log::info!("head transform {:?}", head_tf);
        *tf = head_tf;
    }
}

fn sync_hands_tf(
    mut hands_tf_q: Query<(&Hand, &mut Transform), With<Hand>>,
    xr_ctx: NonSend<WebXrContext>,
    frame: NonSend<web_sys::XrFrame>,
) {
    let reference_space = &xr_ctx.space_info.0;
    let input_sources = frame.session().input_sources();
    let (left_input, right_input) = (input_sources.get(0), input_sources.get(1));

    if let Some(is) = left_input {
        // bevy_log::info!("syncing left hand");
        let hand_pose = frame
            .get_pose(&is.grip_space().unwrap(), reference_space)
            .unwrap();
        let hand_tf: bevy_transform::components::Transform = hand_pose.transform().xr_into();

        for (hand, mut tf) in hands_tf_q.iter_mut().filter(|h| h.0 == &Hand::Left) {
            *tf = Transform {
                translation: bevy_math::Vec3::new(
                    hand_tf.translation.x,
                    hand_tf.translation.y,
                    hand_tf.translation.z,
                ),
                rotation: hand_tf.rotation,
                scale: hand_tf.scale,
            };
            // *tf = hand_tf;
            // tf.translation *= 10.0;
            bevy_log::info!("left hand transform {:?}", tf);
        }
    }

    if let Some(is) = right_input {
        // bevy_log::info!("syncing right hand");
        let hand_pose = frame
            .get_pose(&is.grip_space().unwrap(), reference_space)
            .unwrap();
        // let hand_tf = hand_pose.transform().xr_into();
        let hand_tf: bevy_transform::components::Transform = hand_pose.transform().xr_into();

        for (hand, mut tf) in hands_tf_q.iter_mut().filter(|h| h.0 == &Hand::Right) {
            *tf = Transform {
                translation: bevy_math::Vec3::new(
                    hand_tf.translation.x,
                    -hand_tf.translation.y,
                    hand_tf.translation.z,
                ),
                rotation: hand_tf.rotation,
                scale: hand_tf.scale,
            };
            // *tf = hand_tf;
            // tf.translation *= 10.0;
            bevy_log::info!("right hand transform {:?}", tf);
        }
    }
}

fn setup_webxr_pawn(
    xr_ctx: NonSend<WebXrContext>,
    frame: NonSend<web_sys::XrFrame>,
    id: Res<FramebufferUuid>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let reference_space = &xr_ctx.space_info.0;
    let viewer_pose = frame.get_viewer_pose(&reference_space).unwrap();

    let head_tf = viewer_pose.transform().xr_into();

    let views: Vec<web_sys::XrView> = viewer_pose
        .views()
        .iter()
        .map(|view| view.unchecked_into::<web_sys::XrView>())
        .collect();

    let left_eye: &web_sys::XrView = views
        .iter()
        .find(|view| view.eye() == web_sys::XrEye::Left)
        .unwrap();

    let left_tf: Transform = left_eye.transform().xr_into();

    let right_eye: &web_sys::XrView = views
        .iter()
        .find(|view| view.eye() == web_sys::XrEye::Right)
        .unwrap();

    let right_tf: Transform = right_eye.transform().xr_into();

    let id = id.0;
    let base_layer: web_sys::XrWebGlLayer = frame.session().render_state().base_layer().unwrap();

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

    let uv_debug_texture = {
        const TEXTURE_SIZE: usize = 8;

        let mut palette: [u8; 32] = [
            255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102,
            255, 198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
        ];

        let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
        for y in 0..TEXTURE_SIZE {
            let offset = TEXTURE_SIZE * y * 4;
            texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
            palette.rotate_right(4);
        }

        Image::new_fill(
            Extent3d {
                width: TEXTURE_SIZE as u32,
                height: TEXTURE_SIZE as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &texture_data,
            TextureFormat::Rgba8UnormSrgb,
        )
    };

    let cube_size = 0.05;
    let cube_handle = meshes.add(Mesh::from(shape::Box::new(cube_size, cube_size, cube_size)));
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture)),
        ..default()
    });

    // let controller_gltf = asset_server.load("../../3d_model/controller.gltf#Scene0");

    commands
        .spawn((
            SpatialBundle {
                transform: head_tf,
                visibility: Visibility { is_visible: true },
                ..default()
            },
            HeadMarker,
        ))
        .with_children(|head| {
            head.spawn((
                Camera3dBundle {
                    camera_3d: Camera3d { ..default() },
                    camera: Camera {
                        target: RenderTarget::TextureView(id),
                        viewport: Some(left_viewport),
                        ..default()
                    },
                    transform: left_tf,
                    ..default()
                },
                LeftEyeMarker,
            ));
            head.spawn((
                Camera3dBundle {
                    camera_3d: Camera3d {
                        //Viewport does not affect ClearColor, so we set the right camera to a None Clear Color
                        clear_color: ClearColorConfig::None,
                        ..default()
                    },
                    camera: Camera {
                        target: RenderTarget::TextureView(id),
                        priority: 1,
                        viewport: Some(right_viewport),
                        ..default()
                    },
                    transform: right_tf,
                    ..default()
                },
                RightEyeMarker,
            ));
        });

    commands.spawn((
        Hand::Left,
        PbrBundle {
            mesh: cube_handle.clone(),
            material: debug_material.clone(),
            visibility: Visibility { is_visible: true },
            ..default()
        },
    ));

    commands.spawn((
        Hand::Right,
        PbrBundle {
            mesh: cube_handle.clone(),
            material: debug_material.clone(),
            visibility: Visibility { is_visible: true },
            ..default()
        },
    ));
    // .with_children(|head| {
    //     head.spawn((
    //         Hand::Left,
    //         PbrBundle {
    //             mesh: cube_handle.clone(),
    //             material: debug_material.clone(),
    //             visibility: Visibility { is_visible: true },
    //             ..default()
    //         }
    //     ));
    // })
    // .with_children(|head| {
    //     head.spawn((
    //         Hand::Right,
    //         PbrBundle {
    //             mesh: cube_handle.clone(),
    //             material: debug_material.clone(),
    //             visibility: Visibility { is_visible: true },
    //             ..default()
    //         }
    //     ));
    // });
    bevy_log::info!("finished setup");
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
    let layer_init = web_sys::XrWebGlLayerInit::new();

    let xr_gl_layer = web_sys::XrWebGlLayer::new_with_web_gl2_rendering_context_and_layer_init(
        &webxr_context.session,
        &webgl2_context,
        &layer_init,
    )
    .unwrap();

    let mut render_state_init = web_sys::XrRenderStateInit::new();
    render_state_init
        .depth_near(0.001)
        .base_layer(Some(&xr_gl_layer));

    webxr_context
        .session
        .update_render_state_with_state(&render_state_init);

    world.insert_resource(RenderDevice::from(Arc::new(device)));
    world.insert_resource(RenderQueue(Arc::new(queue)));
    world.insert_resource(RenderAdapterInfo(adapter_info));
    world.insert_non_send_resource(webxr_context);
    let id = Uuid::new_v4();
    world.insert_resource(FramebufferUuid(id));
}

/// System that updates `ManualTextureViews` with the new `WebGlFramebuffer`
fn update_manual_texture_views(
    frame: bevy_ecs::prelude::NonSend<web_sys::XrFrame>,
    device: Res<RenderDevice>,
    framebuffer_uuid: Res<FramebufferUuid>,
    mut manual_tex_view: ResMut<ManualTextureViews>,
) {
    let base_layer: XrWebGlLayer = frame.session().render_state().base_layer().unwrap();

    //Reflect hack because base_layer.framebuffer is technically null
    let framebuffer: web_sys::WebGlFramebuffer =
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
    type XrFrameHandler = Closure<dyn FnMut(f64, web_sys::XrFrame)>;
    let f: Rc<RefCell<Option<XrFrameHandler>>> = Rc::new(RefCell::new(None));
    let g: Rc<RefCell<Option<XrFrameHandler>>> = f.clone();

    *g.borrow_mut() = Some(Closure::new(move |_time: f64, frame: web_sys::XrFrame| {
        app.world.insert_non_send_resource(frame.clone());

        let webxr_context = app.world.get_non_send_resource::<WebXrContext>().unwrap();

        let (space, space_type) = webxr_context.space_info.clone();
        // update the current frame inside tracking soource
        let tracking_source = TrackingSource::new(space, space_type, frame.clone());
        // Resource used to track poses
        app.world
            .insert_resource(bevy_xr::XrTrackingSource::new(Box::new(tracking_source)));

        app.update();

        let session = frame.session();
        session.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }));
    session.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref());
}

pub fn create_view_from_device_framebuffer(
    device: &wgpu::Device,
    framebuffer: web_sys::WebGlFramebuffer,
    base_layer: &web_sys::XrWebGlLayer,
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

#[derive(Component, PartialEq, Eq, Debug)]
pub enum Hand {
    Left,
    Right,
}
