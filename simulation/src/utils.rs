use crate::team::Team;

#[must_use]
pub fn get_combo_multiplier(combo: u32) -> f32 {
    (combo * combo) as f32 * 0.01 + 1.0
}

#[must_use]
pub fn current_and_enemy<const N: usize>(teams: &mut [Team; N], i: usize) -> (&mut Team, &mut Team) {
    assert!(N == 2 && (i == 0 || i == 1));
    let (left, right) = teams.split_at_mut(1);
    if i == 0 {
        (&mut left[0], &mut right[0])
    } else {
        (&mut right[0], &mut left[0])
    }
}
