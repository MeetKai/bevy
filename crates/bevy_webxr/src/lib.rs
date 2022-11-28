use bevy_app::App;
use bevy_app::Plugin;
use bevy_ecs::schedule::IntoSystemDescriptor;
use bevy_ecs::system::Commands;
use bevy_ecs::system::Res;
use bevy_ecs::system::ResMut;
use bevy_ecs::system::Resource;
use bevy_log::info;
use bevy_math::UVec2;
use bevy_math::Vec3;
use bevy_render::camera::Camera;
use bevy_render::camera::ManualTextureViews;
use bevy_render::camera::RenderTarget;
use bevy_render::prelude::Color;
use bevy_render::renderer::RenderDevice;
use bevy_utils::default;
use bevy_utils::Uuid;
use js_sys::Boolean;
use raw_window_handle;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::XrWebGlLayer;

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
    // adapter: Adapter,
    // device: Arc<Device>,
    // queue: Queue,
}

impl WebXrContext {
    /// Get a WebXrContext, you must do this in an async function, so you have to call this before `bevy_app::App::run()` in an async main fn and insett it
    pub async fn get_context(mode: bevy_xr::XrSessionMode) -> Result<Self, JsValue> {
        // gloo_console::log!("beginning of get context");
        let mode = mode.xr_into();
        let window = gloo_utils::window();
        let navigator = window.navigator();
        let xr_system = navigator.xr();
        // gloo_console::log!("pres esssion supported");

        let session_supported =
            JsFuture::from(xr_system.is_session_supported(web_sys::XrSessionMode::ImmersiveVr))
                .await?
                .dyn_into::<Boolean>()?
                .value_of();

        // gloo_console::log!("post session supported");

        if !session_supported {
            return Err("XrSessionMode not supported.".into());
        }

        let session = JsFuture::from(xr_system.request_session(mode))
            .await?
            .dyn_into::<web_sys::XrSession>()?;
        // gloo_console::log!("pre-canvas");

        let canvas = Canvas::default();
        // gloo_console::log!("psot canvas");

        Ok(WebXrContext { session, canvas })
    }
}

#[derive(Default)]
pub struct WebXrPlugin;

impl Plugin for WebXrPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        bevy_log::info!("in webxr");
        app.set_runner(webxr_runner);
        app.add_startup_system(setup);
        app.add_system(render_webxr.label("render_webxr"));
    }
}

fn setup(mut commands: Commands) {
    let left_id = Uuid::new_v4();
    commands.insert_resource(FramebufferUuids { left: left_id });
    commands.spawn((bevy_core_pipeline::core_3d::Camera3dBundle {
        camera_3d: bevy_core_pipeline::core_3d::Camera3d {
            // clear_color: bevy_core_pipeline::clear_color::ClearColorConfig::Custom(Color::BLUE),
            ..default()
        },
        camera: Camera {
            target: RenderTarget::TextureView(left_id),
            ..default()
        },
        transform: bevy_transform::components::Transform::from_translation(Vec3::new(
            0.0, 0.0, 15.0,
        ))
        .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    },));
}

fn render_webxr(
    mut commands: Commands,
    frame: bevy_ecs::prelude::NonSend<web_sys::XrFrame>,
    device: Res<RenderDevice>,
    queue: Res<bevy_render::renderer::RenderQueue>,
    framebuffer_uuids: Res<FramebufferUuids>,
    mut manual_tex_view: ResMut<ManualTextureViews>,
) {
    let base_layer: XrWebGlLayer = frame.session().render_state().base_layer().unwrap();

    // info!("post base layer");

    let framebuffer: web_sys::WebGlFramebuffer =
        js_sys::Reflect::get(&base_layer, &"framebuffer".into())
            .unwrap()
            .into();

    // info!("post framebuffer");
    let device = device.wgpu_device();
    // info!("device");
    let framebuffer_colour_attachment: VrFramebufferTexture = create_view_from_device_framebuffer(
        device,
        framebuffer.clone(),
        &base_layer,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        "device framebuffer (colour)",
    );
    let left_id = framebuffer_uuids.left;
    let resolution = UVec2::new(
        base_layer.framebuffer_width(),
        base_layer.framebuffer_height(),
    );
    manual_tex_view.insert(
        left_id,
        (framebuffer_colour_attachment.view.into(), resolution),
    );
}

#[derive(Resource)]
pub struct FramebufferUuids {
    pub left: Uuid,
}

fn webxr_runner(mut app: App) {
    let webxr_context = app.world.get_non_send_resource::<WebXrContext>().unwrap();
    let session = webxr_context.session.clone();
    type XrFrameHandler = Closure<dyn FnMut(f64, web_sys::XrFrame)>;
    let f: Rc<RefCell<Option<XrFrameHandler>>> = Rc::new(RefCell::new(None));
    let g: Rc<RefCell<Option<XrFrameHandler>>> = f.clone();
    let closure_session = session.clone();
    *g.borrow_mut() = Some(Closure::new(move |_time: f64, frame: web_sys::XrFrame| {
        //Tick Bevy World
        app.world.insert_non_send_resource(frame.clone());
        app.update();

        let session = frame.session();

        session.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }));

    session.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref());
}

pub struct Canvas {
    inner: web_sys::HtmlCanvasElement,
    id: u32,
}

impl Canvas {
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
