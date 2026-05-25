use serde::{Deserialize, Serialize};
use simulation::PlayerInput;

#[derive(Serialize, Deserialize, Clone)]
pub struct NetInput {
    jump: bool,
    up: bool,
    left: bool,
    right: bool,
    slam: bool,
    dash: bool,
    light: bool,
    normal: bool,
    parry: bool,
}

impl NetInput {
    pub fn to_net(input: &PlayerInput) -> NetInput {
        NetInput {
            jump: input.jump,
            up: input.up,
            left: input.left,
            right: input.right,
            slam: input.slam,
            dash: input.dash,
            light: input.light,
            normal: input.normal,
            parry: input.parry,
        }
    }

    pub fn from_net(&self, input: &mut PlayerInput) {
        input.jump = self.jump;
        input.up = self.up;
        input.left = self.left;
        input.right = self.right;
        input.slam = self.slam;
        input.dash = self.dash;
        input.light = self.light;
        input.normal = self.normal;
        input.parry = self.parry;
    }
}
