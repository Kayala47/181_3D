#![allow(dead_code)]

use color_eyre::eyre::Result;

mod engine;
mod image;
mod input;
mod types;
mod camera;
mod vulkan;
mod renderer;
use types::*;

const DT:f32 = 1.0/60.0;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut engine = engine::Engine::new(engine::WindowSettings::default());

    engine.set_camera(camera::Camera::look_at(Vec3::new(0.,0.,-10.), Vec3::zero(), Vec3::unit_y()));
    let tex = engine.load_texture(std::path::Path::new("content/robot.png"))?;
    let mesh = engine.load_mesh(std::path::Path::new("content/characterSmall.fbx"),0.1)?;
    let model = engine.create_model(&mesh, &tex);

    engine.create_game_object(
        Some(&model),
        Isometry3::new(
            Vec3::new(0.0, -12.5, 25.0),
            Rotor3::identity()
        ),
    );

    engine.play(|engine| {
        for obj in engine.objects_mut() {
            obj.move_by(Vec3::new(1.0, 1.0, 0.0) * DT);
        }
    })
}
