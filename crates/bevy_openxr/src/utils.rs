use openxr::{sys, Instance, Session};

use crate::{OpenXrContext, OpenXrSession};

pub fn increase_refresh_rate(instance: &Instance, session: sys::Session) {
    instance.exts().fb_display_refresh_rate.map(|display_fps| {
        let mut fps = 0f32;
        unsafe { (display_fps.get_display_refresh_rate)(session, &mut fps) };
        bevy_log::info!("got refresh rate: {}", fps);
        let mut refresh_rates = [0f32; 10];
        let mut out_count = 0u32;
        let rates = unsafe {
            (display_fps.enumerate_display_refresh_rates)(
                session,
                refresh_rates.len() as u32,
                &mut out_count,
                refresh_rates.as_mut_ptr(),
            )
        };
        bevy_log::info!(
            "available refresh rates: {:?}",
            &refresh_rates[..out_count as usize]
        );
        if fps < 90. {
            let res = unsafe { (display_fps.request_display_refresh_rate)(session, 90.) };
            bevy_log::info!("requested refresh rate {}, result: {:?}", 90., res);
        }
    });
}
