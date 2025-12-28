#[must_use]
pub fn approach(value: f32, target: f32, step: f32) -> f32 {
    if value > target {
        (value - step).max(target)
    } else if value < 0.0 {
        (value + step).min(target)
    } else {
        target
    }
}

