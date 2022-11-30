use bevy_app::{App, Plugin, StartupStage};
use bevy_ecs::prelude::{Component, World};
use bevy_ecs::{
    schedule::IntoSystemDescriptor,
    system::{Commands, Res, ResMut, Resource},
};
use bevy_math::{UVec2, Vec3};
use bevy_render::renderer::{RenderAdapterInfo, RenderQueue};
use bevy_render::{
    camera::{Camera, ManualTextureViews, RenderTarget, Viewport},
    renderer::RenderDevice,
};
use bevy_utils::{default, Uuid};
use js_sys::Boolean;
use raw_window_handle;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::XrWebGlLayer;
use wgpu::{Adapter, Device, Queue};

// WebXR <-> Bevy XR Conversion
pub(crate) trait XrFrom<T> {
    fn xr_from(_: T) -> Self;
}

pub(crate) trait XrInto<T> {
    fn xr_into(self) -> T;
}

impl<T, U> XrInto<U> for T
where
    U: XrFrom<T>,
{
    fn xr_into(self) -> U {
        U::xr_from(self)
    }
}

// XR Conversion Impls

impl XrFrom<web_sys::XrSessionMode> for bevy_xr::XrSessionMode {
    fn xr_from(mode: web_sys::XrSessionMode) -> Self {
        match mode {
            web_sys::XrSessionMode::Inline => bevy_xr::XrSessionMode::InlineVR,
            web_sys::XrSessionMode::ImmersiveVr => bevy_xr::XrSessionMode::ImmersiveVR,
            web_sys::XrSessionMode::ImmersiveAr => bevy_xr::XrSessionMode::ImmersiveAR,
            _ => panic!("Invalid XrSessionMode"),
        }
    }
}

impl XrFrom<bevy_xr::XrSessionMode> for web_sys::XrSessionMode {
    fn xr_from(web_xr: bevy_xr::XrSessionMode) -> Self {
        match web_xr {
            bevy_xr::XrSessionMode::ImmersiveVR => web_sys::XrSessionMode::ImmersiveVr,
            bevy_xr::XrSessionMode::ImmersiveAR => web_sys::XrSessionMode::ImmersiveAr,
            bevy_xr::XrSessionMode::InlineVR => web_sys::XrSessionMode::Inline,
            //TODO: remove bevy_xr::XrSessionMode::InlineAr?
            bevy_xr::XrSessionMode::InlineAR => web_sys::XrSessionMode::Inline,
        }
    }
}

pub struct WebXrContext {
    pub session: web_sys::XrSession,
    pub canvas: Canvas,
}

impl WebXrContext {
    /// Get a WebXrContext, you must do this in an async function, so you have to call this before `bevy_app::App::run()` in an async main fn and insett it
    pub async fn get_context(mode: bevy_xr::XrSessionMode) -> Result<Self, JsValue> {
        let mode = mode.xr_into();
        let window = gloo_utils::window();
        let navigator = window.navigator();
        let xr_system = navigator.xr();

        let session_supported =
            JsFuture::from(xr_system.is_session_supported(web_sys::XrSessionMode::ImmersiveVr))
                .await?
                .dyn_into::<Boolean>()?
                .value_of();

        if !session_supported {
            return Err("XrSessionMode not supported.".into());
        }

        let session = JsFuture::from(xr_system.request_session(mode))
            .await?
            .dyn_into::<web_sys::XrSession>()?;

        let canvas = Canvas::default();

        Ok(WebXrContext { session, canvas })
    }
}

#[derive(Default)]
pub struct WebXrPlugin;

impl Plugin for WebXrPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        bevy_log::info!("in webxr");
        app.set_runner(webxr_runner);
        setup(&mut app.world);
        app.add_system(update_manual_texture_views.label("render_webxr"));
    }
}

pub async fn initialize_webxr() -> InitializedState {
    let webxr_context = WebXrContext::get_context(bevy_xr::XrSessionMode::ImmersiveVR)
        .await
        .unwrap();

    let webgl2_context = webxr_context.canvas.create_webgl2_context();

    // WGpu Setup
    let instance = wgpu::Instance::new(wgpu::Backends::GL);

    let surface = unsafe { instance.create_surface(&webxr_context.canvas) };

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("No suitable GPU adapters found on the system!");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("device"),
                features: adapter.features(),
                limits: adapter.limits(),
            },
            None,
        )
        .await
        .expect("Unable to find a suitable GPU adapter!");

    wasm_bindgen_futures::JsFuture::from(webgl2_context.make_xr_compatible())
        .await
        .expect("Failed to make the webgl context xr-compatible");

    InitializedState {
        webgl2_context,
        webxr_context,
        adapter,
        device,
        queue,
    }
}

#[derive(Resource)]
pub struct InitializedState {
    webgl2_context: web_sys::WebGl2RenderingContext,
    webxr_context: WebXrContext,
    adapter: Adapter,
    device: Device,
    queue: Queue,
}

unsafe impl Send for InitializedState {}

unsafe impl Sync for InitializedState {}

fn setup(world: &mut World) {
    let InitializedState {
        webgl2_context,
        webxr_context,
        adapter,
        device,
        queue,
    } = world.remove_resource().unwrap();
    let adapter_info = adapter.get_info();
    let mut layer_init = web_sys::XrWebGlLayerInit::new();

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
    mut commands: Commands,
    frame: bevy_ecs::prelude::NonSend<web_sys::XrFrame>,
    device: Res<RenderDevice>,
    queue: Res<bevy_render::renderer::RenderQueue>,
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

/// Resource that contains the `Uuid` corresponding to WebGlFramebuffer
#[derive(Resource)]
pub struct FramebufferUuid(pub Uuid);

/// Bevy runner that works with
fn webxr_runner(mut app: App) {
    let webxr_context = app.world.get_non_send_resource::<WebXrContext>().unwrap();
    let session = webxr_context.session.clone();
    type XrFrameHandler = Closure<dyn FnMut(f64, web_sys::XrFrame)>;
    let f: Rc<RefCell<Option<XrFrameHandler>>> = Rc::new(RefCell::new(None));
    let g: Rc<RefCell<Option<XrFrameHandler>>> = f.clone();
    let closure_session = session.clone();
    *g.borrow_mut() = Some(Closure::new(move |_time: f64, frame: web_sys::XrFrame| {
        app.world.insert_non_send_resource(frame.clone());
        app.update();

        let session = frame.session();
        session.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }));
    session.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref());
}

/// Wrapper for `HtmlCanvasElement`
pub struct Canvas {
    inner: web_sys::HtmlCanvasElement,
    id: u32,
}

impl Canvas {
    /// Create new Canvas
    pub fn new_with_id(id: u32) -> Self {
        let canvas: web_sys::HtmlCanvasElement = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("canvas")
            .unwrap()
            .unchecked_into();

        let body = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap();

        canvas
            .set_attribute("data-raw-handle", &id.to_string())
            .unwrap();

        body.append_child(&web_sys::Element::from(canvas.clone()))
            .unwrap();

        Self { inner: canvas, id }
    }

    pub fn create_webgl2_context(
        &self,
        // options: ContextCreationOptions,
    ) -> web_sys::WebGl2RenderingContext {
        let js_gl_attribs = js_sys::Object::new();
        js_sys::Reflect::set(
            &js_gl_attribs,
            &"xrCompatible".into(),
            &wasm_bindgen::JsValue::TRUE,
        )
        .expect("Failed to set xrCompatible");
        // WebGL silently ignores any stencil writing or testing if this is not set.
        // (Atleast on Chrome). What a fantastic design decision.
        // js_sys::Reflect::set(
        //     &js_gl_attribs,
        //     &"stencil".into(),
        //     &wasm_bindgen::JsValue::from_bool(options.stencil),
        // )
        // .expect("Failed to set stencil");

        self.inner
            .get_context_with_context_options("webgl2", &js_gl_attribs)
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::WebGl2RenderingContext>()
            .unwrap()
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new_with_id(0)
    }
}

unsafe impl raw_window_handle::HasRawDisplayHandle for Canvas {
    fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        raw_window_handle::RawDisplayHandle::Web(raw_window_handle::WebDisplayHandle::empty())
    }
}

unsafe impl raw_window_handle::HasRawWindowHandle for Canvas {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        let mut web = raw_window_handle::WebWindowHandle::empty();
        web.id = self.id;

        raw_window_handle::RawWindowHandle::Web(web)
    }
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
