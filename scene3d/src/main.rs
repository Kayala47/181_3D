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
use serde_json;
use crate::engine::*;
use std::fs::File;

const DT:f32 = 1.0/60.0;

fn main() -> Result<()> {
    color_eyre::install()?;
    let world: World = engine::World::new(0, 7);
    let mut engine = engine::Engine::new(engine::WindowSettings::default(), world);

    engine.set_camera(camera::Camera::look_at(Vec3::new(0.,0.,-10.), Vec3::zero(), Vec3::unit_y()));
    let tex = engine.load_texture(std::path::Path::new("content/robot.png"))?;
    let mesh = engine.load_mesh(std::path::Path::new("content/characterSmall.fbx"),0.1)?;
    let model = engine.create_model(&mesh, &tex);
    let mut trf = Isometry3::new(
        Vec3::new(0.0, -30., 25.0),
        Rotor3::identity()
    );

    engine.create_game_object(
        Some(&model),
        trf
    );

    let mut i = 1.0;
    let file = File::open("src/rooms.json").unwrap();
    let json: serde_json::Value = serde_json::from_reader(file).unwrap();
    let rooms = json.get("rooms").unwrap();

    for room in rooms.as_array().unwrap().iter(){
        trf.append_translation(Vec3::new(i, 0.,i));
        i+=4.0;
        let room_id= room["id"].as_i64().unwrap() as usize;
        let objects = room["objects"].as_array().unwrap().iter();
        let connected_rooms = room["connected_rooms"].as_array().unwrap();
        let connected_rooms_arr: [usize; 4] = [
                                                connected_rooms[0].as_i64().unwrap() as usize,
                                                connected_rooms[1].as_i64().unwrap() as usize,
                                                connected_rooms[2].as_i64().unwrap() as usize,
                                                connected_rooms[3].as_i64().unwrap() as usize,
                                            ];
        let mut key_vec = vec![];
        for object in objects{
            let roomid= object["roomid"].as_i64().unwrap() as usize;
            let new_key = Key::new_key(roomid, Some(&model), trf);
            key_vec.push(new_key);
        }

        engine.get_world().add_room(room_id, key_vec, connected_rooms_arr,Some(&model), trf)
    }

    engine.play(|engine| {
        
        let input = engine.get_input();

        let mut move_by = Vec3::new(0.0, 0.0, 0.0);
        if input.is_key_down(winit::event::VirtualKeyCode::Down) {
            move_by.z -= 1.0;
        }
        else if input.is_key_down(winit::event::VirtualKeyCode::Left) {
            move_by.x += 1.0;
        }
        else if input.is_key_down(winit::event::VirtualKeyCode::Right) {
            move_by.x -= 1.0;
        }
        else if input.is_key_down(winit::event::VirtualKeyCode::Up) {
            move_by.z += 1.0;
        }

        engine.get_camera().move_eye(move_by);
        engine.get_camera().move_at(move_by);
        
        let obj = engine.objects_mut().next().unwrap();
        obj.move_by(move_by);
    })
}
