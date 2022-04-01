use crate::types::*;
pub struct Camera {
    eye:Vec3,
    at:Vec3,
    up:Vec3,
    fov:f32,
    ratio:f32,
    z_far:f32
}
impl Camera {
    pub fn look_at(eye:Vec3, at:Vec3, up:Vec3) -> Camera {
        Camera{eye, at, up, fov:PI/2.0, ratio:4.0/3.0, z_far:1000.0}
    }
    
    pub fn as_matrix(&self) -> Mat4 {
        // projection * view
        let proj = ultraviolet::projection::perspective_vk(self.fov, self.ratio, 0.01, self.z_far);
        proj * Mat4::look_at(self.eye, self.at, self.up)
    }
    
    pub fn move_at(&mut self, move_at: Vec3) {
         self.at.x += move_at.x;
         self.at.y += move_at.y;
         self.at.z += move_at.z;
    }

    pub fn move_eye(&mut self, move_eye: Vec3) {
        self.eye.x += move_eye.x;
        self.eye.y += move_eye.y;
        self.eye.z += move_eye.z;
   }
}
