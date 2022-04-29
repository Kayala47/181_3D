#![allow(dead_code)]

use frenderer::animation::{AnimationSettings, AnimationState};
use frenderer::assets::{AnimRef, Texture};
use frenderer::camera::{Camera, FPCamera};
use frenderer::renderer::textured::Model;
use frenderer::types::*;
use frenderer::{Engine, Key, MousePos, Result, WindowSettings};
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::rc::Rc;

const DT: f64 = 1.0 / 60.0;

const GRAB_THRESHOLD: f32 = 100.0;
const COLLIS_THRESHHOLD: f32 = 1.0;
const ROOM_RADIUS: f32 = 50.0; //not the right word, but half the length.
const DOOR_THRESHOLD: f32 = 5.0; // if within this distance of a door, need to have a key

const WALL_X: f32 = 0.1 * 100.0;
const WALL_Y: f32 = 1.0 * 100.0;
const WALL_Z: f32 = 0.5 * 100.0;

const DOOR_X: f32 = 0.1 * 100.0;
const DOOR_Y: f32 = 0.33 * 100.0;
const DOOR_Z: f32 = 0.25 * 100.0;

struct Plane {
    // A normal, has to be a unit vector
    n: Vec3,
    // And a distance of how far along the normal this is
    d: f32,
}

struct Wall {
    trf: Similarity3,
    wall: Rc<Flat>,
    door: Option<Rc<Flat>>,
}

impl Wall {
    fn shape(&self) -> Plane {
        Plane {
            n: self.wall.trf.rotation * Vec3::unit_y(),
            d: self.wall.trf.translation.y + self.trf.scale,
        }
    }

    fn get_models(&self) -> (&Flat, &Option<Rc<Flat>>) {
        (&self.wall, &self.door)
    }

    fn check_collision(&self, other: &Vec3) -> bool {
        //should tell us whether "other" can pass through
        // other can pass when two conditions are met: it hits the wall and also does not hit an open door
        // true tells us to stop the player's velocity

        let wall_coll = distance(self.wall.trf.translation, *other) <= COLLIS_THRESHHOLD;

        let door_coll = match &self.door {
            None => false,
            Some(d) => {
                if distance(d.trf.translation, *other) > COLLIS_THRESHHOLD {
                    false
                } else {
                    true
                }
            }
        };

        wall_coll && door_coll
    }
}

pub struct Player {
    object: GameObject,
    keys_grabbed: Vec<RoomKey>,
    current_room: usize, //id of room
    map: Map,            //so the player knows about the rooms
}

impl Player {
    pub fn grab(&mut self, textureds: &mut Vec<Textured>) {
        //checks if keys are nearby and grabs them

        let curr_pos = self.object.trf.translation;
        dbg!(curr_pos);

        //two steps: filter out the ones that match first, then actually pick them up

        let keys = self.map.room_keys.get_mut(&self.current_room).unwrap();

        let mut tex_iter = textureds.iter();

        if let Some(pos) = tex_iter.position(|t| {
            let textured_pos = t.trf.translation;
            distance(curr_pos, textured_pos) < GRAB_THRESHOLD
        }) {
            println!("attempting to grab something");

            if pos < keys.len() && pos < textureds.len() - 1 {
                let key = keys.remove(pos);
                key.pick_up(self);

                textureds.remove(pos);
            } else {
                println!("not enough elements!");
            }
        }

        dbg!(&self.map.room_keys.get(&self.current_room).unwrap());
        dbg!(textureds);
        dbg!(&self.keys_grabbed);
    }

    pub fn change_room(&mut self, new_roomid: usize) {
        self.current_room = new_roomid;
    }

    pub fn get_pos(&self) -> Vec3 {
        self.object.trf.translation
    }
}

impl fmt::Debug for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Player")
            .field("curr_roomid", &self.current_room)
            .field("pos", &self.object.trf.translation)
            .field("keys_grabbed", &self.keys_grabbed)
            .finish()
    }
}

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
    flats: [usize; 4],           // indices of walls of the room.
    connected_rooms: [usize; 4], //point by ID and N,E,S,W, -1 for no room
}
impl Room {
    pub fn get_flats(&self) -> &[usize] {
        &self.flats
    }
}

pub struct RoomKey {
    starts_roomid: usize, // the room that the key is in.
    opens_roomid: usize,  // the room they open to
    picked_up: bool,
}
impl RoomKey {
    pub fn pick_up(mut self, game_state: &mut Player) {
        self.picked_up = true;
        game_state.keys_grabbed.push(self);
        //TODO: make it disappear   en
    }
}

impl fmt::Debug for RoomKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RoomKey")
            .field("starts_roomid", &self.starts_roomid)
            .field("opens_roomid", &self.opens_roomid)
            .finish()
    }
}

fn gen_key_pair(
    trf: Similarity3,
    model: &Rc<Model>,
    starts: usize,
    opens: usize,
) -> ((usize, usize), Textured) {
    //creates a RoomKey object that is matched with a Textured object

    //TODO: at some point, the transform will take the place of start and we'll need to figure out
    //which room the key is based on the position of the texture

    let texture = Textured {
        trf,
        model: Rc::clone(model),
    };

    let key = RoomKey {
        starts_roomid: starts,
        opens_roomid: opens,
        picked_up: false,
    };

    ((starts, opens), texture)
}

fn multiple_key_pairs(
    trfs: Vec<Similarity3>,
    model: Rc<Model>,
    start_open: Vec<(usize, usize)>,
) -> (Vec<(usize, usize)>, Vec<Textured>) {
    let mut keys = vec![];
    let mut textures = vec![];

    for (trf, (start, open)) in trfs.iter().zip(start_open.iter()) {
        let (k, t) = gen_key_pair(*trf, &model, *start, *open);
        keys.push(k);
        textures.push(t);
    }

    (keys, textures)
}

pub struct Map {
    start_room_id: usize,
    rooms_list: HashMap<usize, Room>,
    room_keys: HashMap<usize, Vec<RoomKey>>, // list of every key found in each room
    end_room_id: usize,
}
impl Map {
    pub fn new(start_room_id: usize, end_room_id: usize) -> Self {
        Self {
            start_room_id,
            rooms_list: HashMap::new(),
            room_keys: HashMap::new(),

            end_room_id,
        }
    }
    pub fn add_room(&mut self, id: usize, flats: [usize; 4], connected_rooms: [usize; 4]) {
        self.rooms_list.insert(
            id,
            Room {
                id: id,
                flats: flats,
                connected_rooms: connected_rooms,
            },
        );
    }

    pub fn add_key(&mut self, starts_roomid: usize, opens_roomid: usize) {
        let key = RoomKey {
            starts_roomid,
            opens_roomid,
            picked_up: false,
        };

        match self.room_keys.get_mut(&starts_roomid) {
            Some(l) => {
                l.push(key);
            }
            None => {
                self.room_keys.insert(starts_roomid, vec![key]);
            }
        }
    }

    pub fn add_mult_keys(&mut self, starts_opens: Vec<(usize, usize)>) {
        for (s, o) in starts_opens.iter() {
            self.add_key(*s, *o);
        }
    }

    pub fn get_rooms_list(&mut self) -> &HashMap<usize, Room> {
        &self.rooms_list
    }
}
pub struct GameState {
    keys_grabbed: Vec<RoomKey>,
}

fn distance(v1: Vec3, v2: Vec3) -> f32 {
    (v1 - v2).mag()
}

struct Sprite {
    trf: Isometry3,
    tex: frenderer::assets::TextureRef,
    cel: Rect,
    size: Vec2,
}

impl fmt::Debug for Sprite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sprite")
            .field("trf", &self.trf)
            .field("pos", &self.trf.translation)
            .finish()
    }
}
pub struct World {
    camera: Camera,
    fp_camera: FPCamera,
    things: Vec<GameObject>, // Add keys to things and give them a spinning animation???
    player: Player,
    sprites: Vec<Sprite>,
    flats: Vec<Flat>,
    textured: Vec<Textured>,
}
pub struct Flat {
    trf: Similarity3,
    model: Rc<frenderer::renderer::flat::Model>,
}
pub struct Textured {
    trf: Similarity3,
    model: Rc<frenderer::renderer::textured::Model>,
}

impl fmt::Debug for Textured {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Textured")
            .field("pos", &self.trf.translation)
            .finish()
    }
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

        // let move_z = input.key_axis(Key::Down, Key::Up) as f32;
        // let move_x = input.key_axis(Key::LEFT, Key::RIGHT) as f32;
        let move_z = input.key_axis(Key::S, Key::W) as f32;
        let move_x = input.key_axis(Key::D, Key::A) as f32;
        let grab = input.is_key_released(Key::Space);

        if grab {
            self.player.grab(&mut self.textured);
        }

        let player = &mut self.player.object;

        let MousePos { x: dx, .. } = input.mouse_delta();
        let rot = Rotor3::from_rotation_xz(dx as f32 * (PI / 4.0) * DT as f32);

        player.trf.prepend_rotation(rot);

        player
            .trf
            .prepend_translation(Vec3::new(move_x * 100.0, 0., move_z * 100.0));

        self.fp_camera
            .update(&input, player.trf.translation, player.trf.rotation);
        self.fp_camera.update_camera(&mut self.camera);

        for _s in self.sprites.iter_mut() {
            //s.trf.append_rotation(rot);
            //s.size.x += dscale;
            //s.size.y += dscale;
        }

        for _m in self.flats.iter_mut() {
            //m.trf.append_rotation(rot);
            //m.trf.scale += dscale;
        }
        for _m in self.textured.iter_mut() {
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
        let obj = &self.player.object;
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

    let wall_with_door_closed_model = engine.load_flat(std::path::Path::new(
        "content/walls/wall_with_door_closed.glb",
    ))?;
    let wall_with_door_opened_model = engine.load_flat(std::path::Path::new(
        "content/walls/wall_with_door_opened.glb",
    ))?;
    let wall_no_door_model =
        engine.load_flat(std::path::Path::new("content/walls/wall_no_door.glb"))?;

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

    let mut map = Map::new(0, 5);
    //let file = File::open("content/world.json").unwrap();

    let file = File::open("content/world-2.json").unwrap();

    let json: serde_json::Value = serde_json::from_reader(file).unwrap();
    let rooms = json.get("rooms").unwrap();

    for room in rooms.as_array().unwrap().iter() {
        let room_id = room["id"].as_i64().unwrap() as usize;
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
    for flat in flats.as_array().unwrap().iter() {
        let mut rot = Rotor3::identity();

        //1.57079 Rust was complaining about this value. now is std::f32::consts::FRAC_PI_2

        if !(flat["is_identity"].as_bool().unwrap()) {
            rot = Rotor3::from_rotation_xz(std::f32::consts::FRAC_PI_2)
        }

        let x = flat["x"].as_f64().unwrap() as f32;
        let y = -15.0;
        let z = flat["z"].as_f64().unwrap() as f32;

        let model = {
            if flat["door"].as_i64().unwrap() as i32 == 0 {
                // Wall model without a door
                wall_no_door_model.clone()
            } else if flat["door"].as_i64().unwrap() as i32 == 1 {
                // Wall model with an open door
                wall_with_door_opened_model.clone()
            } else if flat["door"].as_i64().unwrap() as i32 == 2 {
                // Wall model with a locked door
                wall_with_door_closed_model.clone()
            } else {
                panic!("Invalid value for specification of wall.")
            }
        };
        let new_flat = Flat {
            trf: Similarity3::new(Vec3::new(x, y, z), rot, 100.),
            model: model.clone(),
        };
        flats_vec.push(new_flat);
    }

    let player_obj = GameObject {
        trf: Similarity3::new(Vec3::new(-20.0, -15.0, -10.0), Rotor3::identity(), 0.1),
        model,
        animation,
        state: AnimationState { t: 0.0 },
    };

    let key_positions = vec![
        Similarity3::new(Vec3::new(0.0, 0.0, -10.0), Rotor3::identity(), 5.0),
        Similarity3::new(Vec3::new(10.0, 0.0, -10.0), Rotor3::identity(), 5.0),
        Similarity3::new(Vec3::new(0.0, 10.0, -15.0), Rotor3::identity(), 5.0),
    ];

    let (keys, mut key_textureds) =
        multiple_key_pairs(key_positions, marble, vec![(0, 1), (0, 2), (0, 3)]);

    map.add_mult_keys(keys);

    let mut all_textureds = vec![];

    //start w just the floor and then add keys
    // let mut all_textureds = vec![Textured {
    //     trf: Similarity3::new(Vec3::new(0.0, -25.0, 0.0), Rotor3::identity(), 10.0),
    //     model: floor,
    // }];
    // let mut all_textureds = vec![];
    all_textureds.append(&mut key_textureds);
    all_textureds.append(&mut vec![Textured {
        trf: Similarity3::new(Vec3::new(0.0, -25.0, 0.0), Rotor3::identity(), 10.0),
        model: floor,
    }]);
    // For testing purposes

    // let new_flat = Flat {
    //     trf: Similarity3::new(Vec3::new(0.0, -15.0, 0.0), Rotor3::identity(), 100.),
    //     model: wall_with_door_opened_model.clone(),
    // };
    // flats_vec.push(new_flat);
    // let new_flat_2 = Flat {
    //     trf: Similarity3::new(
    //         Vec3::new(100.0, -15.0, 192.0),
    //         Rotor3::from_rotation_xz(1.57079),
    //         100.,
    //     ),
    //     model: wall_no_door_model.clone(),
    // };
    // flats_vec.push(new_flat_2);

    let world = World {
        camera,
        fp_camera,
        things: vec![],
        player: Player {
            object: player_obj,
            keys_grabbed: vec![],
            current_room: map.start_room_id,
            map,
        },
        sprites: vec![],
        flats: flats_vec,
        textured: all_textureds,
    };

    engine.play(world)
}
