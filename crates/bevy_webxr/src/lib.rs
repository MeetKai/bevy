use bevy_app::Plugin;
use js_sys::Boolean;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

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

struct WebXrContext {
    session: web_sys::XrSession,
}

impl WebXrContext {
    /// Get a WebXrContext, you must do this in an async function, so you have to call this before `bevy_app::App::run()` in an async main fn and insett it
    async fn get_context(mode: bevy_xr::XrSessionMode) -> Result<Self, JsValue> {
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

        Ok(WebXrContext { session })
    }
}

struct WebXrPlugin;

impl Plugin for WebXrPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let webxr_context = app
            .world
            .get_non_send_resource_mut::<WebXrContext>()
            .expect("Webxr context has to be inserted before `app.run()`");
    }
}
