use js_sys::Boolean;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::XrSession;

//From Web XR

pub trait FromWebXr<T> {
    fn from_web_xr(web_xr: T) -> Self;
}

pub trait IntoWebXr<T> {
    fn into_web_xr(self) -> T;
}

impl<T, U> IntoWebXr<U> for T
where
    U: FromWebXr<T>,
{
    fn into_web_xr(self) -> U {
        U::from_web_xr(self)
    }
}

impl FromWebXr<web_sys::XrSessionMode> for bevy_xr::XrSessionMode {
    fn from_web_xr(mode: web_sys::XrSessionInit) -> Self {
        //TODO: remove bevy_xr::XrSessionMode::InlineAr?
        match mode {
            web_sys::XrSessionMode::Inline => bevy_xr::XrSessionMode::InlineVR,
            web_sys::XrSessionMode::ImmersiveVr => bevy_xr::XrSessionMode::ImmersiveVR,
            web_sys::XrSessionMode::ImmersiveAr => bevy_xr::XrSessionMode::ImmersiveAR,
        }
    }
}

//From Bevy XR

pub trait FromBevyXr<T> {
    fn from_bevy_xr(web_xr: T) -> Self;
}

pub trait IntoBevyXr<T> {
    fn into_bevy_xr(self) -> T;
}

impl<T, U> IntoBevyXr<U> for T
where
    U: FromBevyXr<T>,
{
    fn into_bevy_xr(self) -> U {
        U::from_bevy_xr(self)
    }
}

struct WebXrContext {}

impl WebXrContext {
    async fn get_context(mode: bevy_xr::XrSessionMode) -> Result<Self, JsValue> {
        let mode = web_sys::XrSessionMode::from(mode);
        let window = gloo_utils::window();
        let navigator = window.navigator();
        let xr_system = navigator.xr();

        let session_supported =
            JsFuture::from(xr_system.is_session_supported(web_sys::XrSessionMode::ImmersiveVr))
                .await?
                .dyn_into::<Boolean>()?
                .value_of();

        let session: XrSession = JsFuture::from(xr_system.request_session(mode))
            .await?
            .dyn_into::<XrSession>()?;
        Ok(WebXrContext {})
    }
}

fn convert_session_mode(bevy_session_mode: bevy_xr::XrSessionMode) -> web_sys::XrSessionMode {
    match bevy_session_mode {
        bevy_xr::XrSessionMode::ImmersiveVR => web_sys::XrSessionMode::ImmersiveVr,
        bevy_xr::XrSessionMode::ImmersiveAR => web_sys::XrSessionMode::ImmersiveAr,
        bevy_xr::XrSessionMode::InlineVR => web_sys::XrSessionMode::Inline,
        bevy_xr::XrSessionMode::InlineAR => web_sys::XrSessionMode::Inline,
    }
}
