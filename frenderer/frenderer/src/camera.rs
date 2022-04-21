use crate::types::*;
use crate::MousePos;
use crate::Input;
const DT: f64 = 1.0 / 60.0;
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub transform: Similarity3,
    pub fov: f32,
    pub ratio: f32,
}
impl Camera {
    pub fn look_at(eye: Vec3, at: Vec3, up: Vec3) -> Camera {
        let iso = Mat4::look_at(eye, at, up).into_isometry();
        Self::from_transform(Similarity3::new(iso.translation, iso.rotation, 1.0))
    }
    pub fn from_transform(s: Similarity3) -> Self {
        Self {
            transform: s,
            fov: PI / 2.0,
            ratio: 4.0 / 3.0,
        }
    }
    pub fn set_ratio(&mut self, r: f32) {
        self.ratio = r;
    }
    pub fn as_matrix(&self) -> Mat4 {
        // projection * view
        let proj = ultraviolet::projection::rh_yup::perspective_reversed_infinite_z_vk(
            self.fov, self.ratio, 0.1,
        );
        proj * self.transform.into_homogeneous_matrix()
    }
    pub fn interpolate(&self, other: &Self, r: f32) -> Self {
        Self {
            transform: self.transform.lerp(&other.transform, r),
            fov: self.fov.lerp(other.fov, r),
            ratio: self.ratio.lerp(other.ratio, r),
        }
    }
}

pub struct FPCamera {
    pub pitch: f32,
    player_pos: Vec3,
    player_rot: Rotor3,
}
impl FPCamera {
    pub fn new() -> Self {
        Self {
            pitch: 0.0,
            player_pos: Vec3::zero(),
            player_rot: Rotor3::identity(),
        }
    }
    pub fn update(&mut self, input: &Input, player_pos: Vec3, player_rot: Rotor3) {
        //let MousePos { y: dy, .. } = input.mouse_delta();
        // self.pitch += DT as f32 * dy as f32 / 10.0;
        // Make sure pitch isn't directly up or down (that would put
        // `eye` and `at` at the same z, which is Bad)
        self.pitch = self.pitch.clamp(-PI / 2.0 + 0.001, PI / 2.0 - 0.001);
        self.player_pos = player_pos;
        self.player_rot = player_rot;
    }
    pub fn update_camera(&self, c: &mut Camera) {
        // The camera's position is offset from the player's position.
        let eye = self.player_pos
        // So, <0, 25, 2> in the player's local frame will need
        // to be rotated into world coordinates. Multiply by the player's rotation:
            + self.player_rot * Vec3::new(0.0, 25.0, 2.0);

        // Next is the trickiest part of the code.
        // We want to rotate the camera around the way the player is
        // facing, then rotate it more to pitch is up or down.

        // We need to turn this rotation into a target vector (at) by
        // picking a point a bit "in front of" the eye point with
        // respect to our rotation.  This means composing two
        // rotations (player and camera) and rotating the unit forward
        // vector around by that composed rotation, then adding that
        // to the camera's position to get the target point.
        // So, we're adding a position and an offset to obtain a new position.
        let at = eye + self.player_rot * Rotor3::from_rotation_yz(self.pitch) * Vec3::unit_z();
        *c = Camera::look_at(eye, at, Vec3::unit_y());
    }
}