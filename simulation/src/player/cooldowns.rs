use crate::utils::tick_timers;

#[derive(Clone)]
pub struct PlayerCooldowns {
    pub dash: f32,
    pub normal: f32,
    pub light: f32,
    pub parry: f32,
}

impl Default for PlayerCooldowns {
    fn default() -> Self {
        Self {
            dash: 0.0,
            normal: 0.0,
            light: 0.0,
            parry: 0.0,
        }
    }
}

impl PlayerCooldowns {
    pub fn tick(&mut self, dt: f32) {
        tick_timers(&mut [
            &mut self.dash,
            &mut self.normal,
            &mut self.light,
            &mut self.parry,
        ], dt);
    }

    pub fn activate_dash(&mut self) {
        self.dash = 3.0;
    }

    pub fn activate_normal(&mut self) {
        self.normal = 0.75;
    }

    pub fn activate_light(&mut self) {
        self.light = 2.0;
    }

    pub fn activate_parry(&mut self) {
        self.parry = 4.0;
    }

    pub fn normal_hit(&mut self) {
        self.normal -= 0.25;
    }

    #[must_use]
    pub fn can_dash(&self) -> bool { self.dash <= 0.0 }

    #[must_use]
    pub fn can_normal(&self) -> bool { self.normal <= 0.0 }

    #[must_use]
    pub fn can_light(&self) -> bool { self.light <= 0.0 }

    #[must_use]
    pub fn can_parry(&self) -> bool { self.parry <= 0.0 }
}
