use crate::{conversion::XrInto, initialization::Canvas};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

///Contains the XrSession and Canvas that is being rendered to
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
                .dyn_into::<js_sys::Boolean>()?
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
