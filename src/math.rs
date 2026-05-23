pub(crate) fn ease_out_cubic(progress: f32) -> f32 {
    1.0 - (1.0 - progress).powi(3)
}

pub(crate) fn lerp(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}
