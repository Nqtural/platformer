#[must_use]
pub fn get_combo_multiplier(combo: u32) -> f32 {
    (combo * combo) as f32 * 0.01 + 1.0
}

pub fn tick_timers(timers: &mut [&mut f32], dt: f32) {
    for t in timers {
        if **t > 0.0 {
            **t -= dt;
        }
        **t = (**t).max(0.0);
    }
}
