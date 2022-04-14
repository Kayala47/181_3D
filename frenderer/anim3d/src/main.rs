#![allow(dead_code)]

use frenderer::animation::{AnimationSettings, AnimationState};
use frenderer::assets::AnimRef;
use frenderer::camera::{Camera, FPCamera};
use frenderer::types::*;
use frenderer::{Engine, Key, Result, WindowSettings};
use std::rc::Rc;
use std::collections::HashMap;
use std::fs::File;

const DT: f64 = 1.0 / 60.0;

pub struct GameObject {
    trf: Similarity3,
    model: Rc<frenderer::renderer::skinned::Model>,
    animation: AnimRef,
    state: AnimationState,
}
impl GameObject {
    fn tick_animation(&mut self) {
        self.state.tick(DT);
    }
}

pub struct Room {
    id: usize,
    flats: [usize; 4], // indices of walls of the room.
    connected_rooms: [usize; 4], //point by ID and N,E,S,W, -1 for no room
}
impl Room {

    pub fn get_flats(&self) -> &[usize] {
        & self.flats
    }

}

pub struct RoomKey {
    starts_roomid: usize, // the room that the key is in.
    opens_roomid: usize, // the room they open to
    sprite_index: usize, // corresponds to the index in the world sprites
    picked_up: bool,
}
impl RoomKey {

    pub fn get_sprite_index(& self) -> & usize{
       & self.sprite_index
    }

    pub fn pick_up(mut self, game_state: &mut GameState){
        self.picked_up = true;
        game_state.keys_grabbed.push(self);
        //TODO: make it disappear   en
    }

}

pub struct Map {
    start_room_id: usize,
    rooms_list: HashMap<usize, Room>,
    room_keys: Vec<RoomKey>,
    end_room_id: usize,
}
impl Map {

    pub fn new(start_room_id: usize, end_room_id: usize ) -> Self {
        Self {
            start_room_id,
            rooms_list: HashMap::new(),
            room_keys: vec![],
            end_room_id
        }
    }
    pub fn add_room(&mut self, id: usize, flats: [usize; 4], connected_rooms: [usize;4]) {
        self.rooms_list.insert(id,Room {
            id: id,
            flats: flats,
            connected_rooms: connected_rooms,
        } );
        
    }

    pub fn get_rooms_list(&mut self) -> &HashMap<usize, Room>  {
        &self.rooms_list
    }

}
pub struct GameState{
    keys_grabbed: Vec<RoomKey>,
}


struct Sprite {
    trf: Isometry3,
    tex: frenderer::assets::TextureRef,
    cel: Rect,
    size: Vec2,
}
struct World {
    camera: Camera,
    fp_camera: FPCamera,
    things: Vec<GameObject>, // Add keys to things and give them a spinning animation???
    player: GameObject,
    sprites: Vec<Sprite>,
    flats: Vec<Flat>,
    textured: Vec<Textured>,
    map: Map,
}
struct Flat {
    trf: Similarity3,
    model: Rc<frenderer::renderer::flat::Model>,
}
struct Textured {
    trf: Similarity3,
    model: Rc<frenderer::renderer::textured::Model>,
}

impl frenderer::World for World {
    fn update(&mut self, input: &frenderer::Input, _assets: &mut frenderer::assets::Assets) {
        //let yaw = input.key_axis(Key::Q, Key::W) * PI / 4.0 * DT as f32;
        //let pitch = input.key_axis(Key::A, Key::S) * PI / 4.0 * DT as f32;
        //let roll = input.key_axis(Key::Z, Key::X) * PI / 4.0 * DT as f32;
        //let dscale = input.key_axis(Key::E, Key::R) * 1.0 * DT as f32;
        //let rot = Rotor3::from_euler_angles(roll, pitch, yaw);

        for obj in self.things.iter_mut() {
            //obj.trf.append_rotation(rot);
            //obj.trf.scale = (obj.trf.scale + dscale).max(0.01);
            // dbg!(obj.trf.rotation);
            obj.tick_animation();
        }

        let move_z = input.key_axis(Key::Down, Key::Up) as f32;
        let move_x = input.key_axis(Key::Right, Key::Left) as f32;
        
        let s = &mut self.player;
        s.trf.append_translation(Vec3::new(move_x, 0., move_z));
        
        self.fp_camera.update(&input, self.player.trf.translation,self.player.trf.rotation);
        self.fp_camera.update_camera(&mut self.camera);

        for s in self.sprites.iter_mut() {
            //s.trf.append_rotation(rot);
            //s.size.x += dscale;
            //s.size.y += dscale;
        }

        for m in self.flats.iter_mut() {
            //m.trf.append_rotation(rot);
            //m.trf.scale += dscale;
        }
        for m in self.textured.iter_mut() {
            //m.trf.append_rotation(rot);
            //sm.trf.scale += dscale;
        }

        //let camera_drot = input.key_axis(Key::Left, Key::Right) * PI / 4.0 * DT as f32;
        //self.camera.transform.prepend_rotation(Rotor3::from_rotation_xz(camera_drot));


    }
    fn render(
        &mut self,
        _a: &mut frenderer::assets::Assets,
        rs: &mut frenderer::renderer::RenderState,
    ) {
        rs.set_camera(self.camera);
        for (obj_i, obj) in self.things.iter_mut().enumerate() {
            rs.render_skinned(obj.model.clone(), obj.animation, obj.state, obj.trf, obj_i);
        }
        for (s_i, s) in self.sprites.iter_mut().enumerate() {
            rs.render_sprite(s.tex, s.cel, s.trf, s.size, s_i);
        }
        let obj = &self.player;
        rs.render_skinned(obj.model.clone(), obj.animation, obj.state, obj.trf, 0);
        for (m_i, m) in self.flats.iter_mut().enumerate() {
            rs.render_flat(m.model.clone(), m.trf, m_i);
        }
        for (t_i, t) in self.textured.iter_mut().enumerate() {
            rs.render_textured(t.model.clone(), t.trf, t_i);
        }
    }
}
fn main() -> Result<()> {
    frenderer::color_eyre::install()?;

    let mut engine: Engine = Engine::new(WindowSettings::default(), DT);

    let camera = Camera::look_at(
        Vec3::new(0., 100., 100.),
        Vec3::new(0., 0., 0.),
        Vec3::new(0., 1., 0.),
    );
    let fp_camera = FPCamera::new();

    let marble_tex = engine.load_texture(std::path::Path::new("content/sphere-diffuse.jpg"))?;
    let marble_meshes = engine.load_textured(std::path::Path::new("content/sphere.obj"))?;
    let marble = engine.create_textured_model(marble_meshes, vec![marble_tex]);
    let floor_tex = engine.load_texture(std::path::Path::new("content/cube-diffuse.jpg"))?;
    let floor_meshes = engine.load_textured(std::path::Path::new("content/floor.obj"))?;
    let floor = engine.create_textured_model(floor_meshes, vec![floor_tex]);
    let king = engine.load_texture(std::path::Path::new("content/king.png"))?;
    let half_wall_model = engine.load_flat(std::path::Path::new("content/wallHalf.glb"))?;
    let tex = engine.load_texture(std::path::Path::new("content/robot.png"))?;
    let meshes = engine.load_skinned(
        std::path::Path::new("content/characterSmall.fbx"),
        &["RootNode", "Root"],
    )?;
    let animation = engine.load_anim(
        std::path::Path::new("content/anim/run.fbx"),
        meshes[0],
        AnimationSettings { looping: true },
        "Root|Run",
    )?;
    assert_eq!(meshes.len(), 1);
    let model = engine.create_skinned_model(meshes, vec![tex]);
    let flat_model = engine.load_flat(std::path::Path::new("content/windmill.glb"))?;

    let mut map = Map::new(0,5);
    let file = File::open("content/world.json").unwrap();
    let json: serde_json::Value = serde_json::from_reader(file).unwrap();
    let rooms = json.get("rooms").unwrap();

    for room in rooms.as_array().unwrap().iter(){
        let room_id= room["id"].as_i64().unwrap() as usize;
        let flats = room["flats"].as_array().unwrap();
        let flats_arr: [usize; 4] = [
            flats[0].as_i64().unwrap() as usize,
            flats[1].as_i64().unwrap() as usize,
            flats[2].as_i64().unwrap() as usize,
            flats[3].as_i64().unwrap() as usize,
        ];
        let connected_rooms = room["connected_rooms"].as_array().unwrap();
        let connected_rooms_arr: [usize; 4] = [
                                                connected_rooms[0].as_i64().unwrap() as usize,
                                                connected_rooms[1].as_i64().unwrap() as usize,
                                                connected_rooms[2].as_i64().unwrap() as usize,
                                                connected_rooms[3].as_i64().unwrap() as usize,
                                            ];

        map.add_room(room_id, flats_arr, connected_rooms_arr);
    }

    let mut flats_vec: Vec<Flat> = vec![];
    let flats = json.get("flats").unwrap();
    for flat in flats.as_array().unwrap().iter(){
        let mut rot= Rotor3::identity();

        if !(flat["is_identity"].as_bool().unwrap()) {
            rot = Rotor3::from_rotation_xz(1.57079)
        }

        let x = flat["x"].as_f64().unwrap() as f32;
        let y = -15.0;
        let z = flat["z"].as_f64().unwrap() as f32;
        
        let new_flat = Flat {
            trf:  Similarity3::new(Vec3::new(x,y,z), rot, 100.),
            model: half_wall_model.clone()
        };
        flats_vec.push(new_flat);
    
    }



    let world = World {
        camera,
        fp_camera,
        things: vec![],
        player: GameObject {
            trf: Similarity3::new(Vec3::new(-20.0, -15.0, -10.0), Rotor3::identity(), 0.1),
            model,
            animation,
            state: AnimationState { t: 0.0 },
        },
        sprites: vec![Sprite {
            trf: Isometry3::new(Vec3::new(20.0, 5.0, -10.0), Rotor3::identity()),
            size: Vec2::new(16.0, 16.0),
            cel: Rect::new(0.5, 0.5, 0.5, 0.5),
            tex: king,
        }],
        flats: flats_vec,
        textured: vec![
            Textured {
                trf: Similarity3::new(Vec3::new(0.0, 0.0, -10.0), Rotor3::identity(), 5.0),
                model: marble,
            },
            Textured {
                trf: Similarity3::new(Vec3::new(0.0, -25.0, 0.0), Rotor3::identity(), 10.0),
                model: floor,
            },
        ],
        map
    };
    engine.play(world)
}
