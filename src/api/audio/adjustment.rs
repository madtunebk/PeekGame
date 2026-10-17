//! IAudioClockAdjustment: real host resampler rate changes for RATEADJUST streams.
use super::*;
unsafe extern "win64" fn set_rate(this: *mut Object, rate: f32) -> H {
    if !rate.is_finite() || !(8000.0..=192000.0).contains(&rate) || rate.fract() != 0.0 {
        return ARG;
    }
    let mut s = unsafe { client::state(this) }.lock().unwrap();
    let Some(stream) = &s.stream else {
        return NOT_INITIALIZED;
    };
    if !s.rate_adjust {
        return NO_INTERFACE;
    }
    if stream.set_rate(rate as u32).is_err() {
        return INVALIDATED;
    }
    s.effective_rate = rate as u32;
    0
}
table!(TABLE, qi, addref, release, set_rate);
