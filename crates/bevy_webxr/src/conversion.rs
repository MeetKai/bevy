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

impl XrFrom<web_sys::XrInputSourceArray> for Vec<web_sys::XrInputSource> {
    fn xr_from(sources: web_sys::XrInputSourceArray) -> Self {
        let mut output = Vec::new();
        for index in 0..sources.length() {
            output.push(sources.get(index).unwrap());
        }
        output
    }
}
