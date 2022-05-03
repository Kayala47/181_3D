#![allow(dead_code)]

use frenderer::animation::{AnimationSettings, AnimationState};
use frenderer::assets::{AnimRef, Texture};
use frenderer::camera::{Camera, FPCamera};
use frenderer::renderer::textured::Model;
use frenderer::types::*;
use frenderer::{Engine, Key, MousePos, Result, WindowSettings};
use kira::{
    arrangement::{Arrangement, LoopArrangementSettings},
    instance::InstanceSettings,
    manager::{AudioManager, AudioManagerSettings},
    sound::SoundSettings,
    Tempo,
};
use russimp::AABB;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::rc::Rc;

const DT: f64 = 1.0 / 60.0;

const GRAB_THRESHOLD: f32 = 100.0;

const WALL_WIDTH: f32 = 3.0; //x
const WALL_HEIGHT: f32 = 1.0 * 100.0; //y
const WALL_THICKNESS: f32 = 0.1 * 100.; //z

const DOOR_WIDTH: f32 = 0.3 * 100.0;
const DOOR_HEIGHT: f32 = 0.7 * 100.0;

const WALL_X: f32 = 0.1 * 100.0;
const WALL_Y: f32 = 1.0 * 100.0;
const WALL_Z: f32 = 0.5 * 100.0;

const DOOR_X: f32 = 0.1 * 100.0;
const DOOR_Y: f32 = 0.33 * 100.0;
const DOOR_Z: f32 = 0.25 * 100.0;

const ROOM_WIDTH: f32 = 300.0;
const ROOM_LENGTH: f32 = 295.0;

const PLAYER_HEIGHT: f32 = WALL_HEIGHT / 2.;

const FIX: Vec2 = Vec2::new(45.0, 45.0);

//let's call this the radius of each
const COLLIS_THRESHHOLD: f32 = (WALL_WIDTH / 2.) + (PLAYER_HEIGHT / 2.);

struct Circle {
    center: Vec2,
    radius: f32,
}

struct Wall {
    wall: Option<AABB2D>,
}

impl fmt::Debug for Wall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Wall")
            .field(
                "wall_pos",
                if let Some(r) = &self.wall {
                    &r.center
                } else {
                    &self.wall
                },
            )
            .finish()
    }
}

struct AABB2D {
    center: Vec2,
    half_widths: Vec2,
    disp_mult: f32,
}

// Find collision given the player's x and z coordinates and current_room.
pub fn handle_collision(player: &mut Player) {
    // find current room
    player.find_current_room();
    let current_room = player.map.rooms_list.get(&player.current_room).unwrap();
    let player_x = player.object.trf.translation.x;
    let player_z = player.object.trf.translation.z;
    let room_bottom_left_corner_x = current_room.bottom_left_corner[0];
    let room_bottom_left_corner_z = current_room.bottom_left_corner[1];
    // check for any overlaps
    if player_x > room_bottom_left_corner_x + ROOM_WIDTH {
        player.object.trf.translation.x = room_bottom_left_corner_x + ROOM_WIDTH - 2.;
    }
    if player_z > room_bottom_left_corner_z + ROOM_LENGTH {
        player.object.trf.translation.z = room_bottom_left_corner_z + ROOM_LENGTH - 2.;
    }
    if player_x < room_bottom_left_corner_x {
        player.object.trf.translation.x = room_bottom_left_corner_x + 2.;
    }
    if player_z < room_bottom_left_corner_z {
        player.object.trf.translation.z = room_bottom_left_corner_z + 2.;
    }
    // update player's coordinates accordingly
}

// fn displacement(c: &Circle, r: &AABB2D) -> Option<Vec3> {
//     // let x_disp = r.half_widths.x + c.radius - (c.center.x - r.center.x).abs();
//     // let z_disp = r.half_widths.y + c.radius - (c.center.y - r.center.y).abs();
//     // if x_disp > 0.0 || z_disp > 0.0 {
//     //     if x_disp < z_disp {
//     //         Some(Vec3::new(x_disp, 0.0, 0.))
//     //     } else {
//     //         Some(Vec3::new(0., 0., z_disp))
//     //     }
//     // } else {
//     //     println!("not displaced");
//     //     None
//     }
impl fmt::Debug for AABB2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AABB2D").field("pos", &self.center).finish()
    }
}

fn displacement(c: &Circle, r: &AABB2D) -> Option<Vec3> {
    let x = c
        .center
        .x
        .clamp(r.center.x - r.half_widths.x, r.center.x + r.half_widths.x);
    let y = c
        .center
        .y
        .clamp(r.center.y - r.half_widths.y, r.center.y + r.half_widths.y);

    let closest_pt = Vec2::new(x, y);
    // let mtv = (closest_pt - c.center).normalized() * (c.radius - closest_pt.mag());
    // dbg!(&mtv);

    if (closest_pt - c.center).mag() <= c.radius {
        //collision!

        // let lt_disp = (c.center.x + c.radius) - (rect.center.x - rect.half_widths.x);
        // let rt_disp = (r.center.x + r.half_widths.x) - (c.center.x - c.radius);

        let x_disp = r.half_widths.x + c.radius - (c.center.x - r.center.x).abs();
        let z_disp = r.half_widths.y + c.radius - (c.center.y - r.center.y).abs();

        // let mut mult = 1.0;
        // if c.center.x < r.center.x || c.center.y < r.center.y {
        //     mult = -1.0;
        // }

        if x_disp < z_disp {
            Some(Vec3::new(x_disp, 0.0, 0.) * r.disp_mult)
        } else {
            Some(Vec3::new(0., 0., z_disp) * r.disp_mult)
        }

        // let x = mtv.x;
        // let y = mtv.y;
        // Some(Vec3::new(x, 0., y))
    } else {
        None
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

        self.find_current_room();

        if self.current_room == 5 {
            println!("\n \n \n \n YOU WIN!!! \n \n \n \n ");
            return;
        }
        // dbg!(curr_pos);

        //two steps: filter out the ones that match first, then actually pick them up

        let mut tex_iter = textureds.iter();

        if let Some(mut pos) = tex_iter.position(|t| {
            let textured_pos = t.trf.translation;
            distance(curr_pos, textured_pos) < GRAB_THRESHOLD
        }) {
            println!("attempting to grab something");

            let og_pos = pos;

            for i in 0 as usize..self.current_room {
                pos -= self.map.room_keys.get(&i).unwrap().len();
            }

            let keys = self.map.room_keys.get_mut(&self.current_room).unwrap();
            dbg!(&keys);

            if pos < keys.len() && pos < textureds.len() - 2 {
                let key = keys.remove(pos);
                key.pick_up(self);

                textureds.remove(og_pos);
            } else {
                println!("not enough elements!");
            }
        }

        // dbg!(&self.map.room_keys.get(&self.current_room).unwrap());
        // dbg!(textureds);
        // dbg!(&self.keys_grabbed);
    }

    pub fn change_room(&mut self, new_roomid: usize) {
        self.current_room = new_roomid;
    }

    fn shape(&self) -> Circle {
        // dbg!(&self.object.trf.translation.x);
        Circle {
            center: Vec2::new(self.object.trf.translation.x, self.object.trf.translation.z)
                * self.object.trf.scale
                * 10.,
            radius: 0.01,
        }
    }

    pub fn find_current_room(&mut self) {
        let room_id = self
            .map
            .find_current_room(self.object.trf.translation.x, self.object.trf.translation.z);
        println!("Current room id is {room_id}");
        self.change_room(room_id);
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
    flats: [usize; 4],            // indices of walls of the room.
    connected_rooms: [usize; 4],  //point by ID and N,E,S,W, -1 for no room
    bottom_left_corner: [f32; 2], // point coordinates for the bottom left corner of the room.
}
impl Room {
    pub fn get_flats(&self) -> &[usize] {
        &self.flats
    }
}

pub struct RoomKey {
    starts_roomid: usize, // the room that the key is in.
    opens_wallid: usize,  // the wall they open to
    picked_up: bool,
}
impl RoomKey {
    pub fn pick_up(mut self, game_state: &mut Player) {
        self.picked_up = true;
        game_state.keys_grabbed.push(self);
    }
}

impl fmt::Debug for RoomKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RoomKey")
            .field("starts_roomid", &self.starts_roomid)
            .field("opens_wallid", &self.opens_wallid)
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
        opens_wallid: opens,
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
    walls: Vec<Wall>,
}
impl Map {
    pub fn new(start_room_id: usize, end_room_id: usize) -> Self {
        Self {
            start_room_id,
            rooms_list: HashMap::new(),
            room_keys: HashMap::new(),
            end_room_id,
            walls: vec![],
        }
    }
    pub fn add_room(
        &mut self,
        id: usize,
        flats: [usize; 4],
        connected_rooms: [usize; 4],
        bottom_left_corner: [f32; 2],
    ) {
        self.rooms_list.insert(
            id,
            Room {
                id: id,
                flats: flats,
                connected_rooms: connected_rooms,
                bottom_left_corner: bottom_left_corner,
            },
        );
    }

    pub fn add_key(&mut self, starts_roomid: usize, opens_wallid: usize) {
        let key = RoomKey {
            starts_roomid,
            opens_wallid,
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

    fn add_walls(&mut self, w: Vec<Wall>) {
        self.walls = w;
    }

    // Finds the current room the player is in given the player's x and z coordinates.
    pub fn find_current_room(&self, x: f32, z: f32) -> usize {
        for room_tuple in self.rooms_list.iter() {
            let room = room_tuple.1;
            if room.bottom_left_corner[0] <= x
                && x <= room.bottom_left_corner[0] + ROOM_WIDTH
                && room.bottom_left_corner[1] <= z
                && z <= room.bottom_left_corner[1] + ROOM_LENGTH
            {
                return *room_tuple.0;
            }
        }
        println!("Player coords: x = {x}, z = {z}");
        return 10000;
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
    open_model: Rc<frenderer::renderer::flat::Model>,
}

impl fmt::Debug for Flat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Flat")
            .field("pos", &self.trf.translation)
            .finish()
    }
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
        let find_room = input.is_key_released(Key::F);

        if find_room {
            self.player.find_current_room();
        }

        if grab {
            self.player.grab(&mut self.textured);
        }

        let player_shape = &self.player.shape();
        let player = &mut self.player.object;

        let MousePos { x: dx, .. } = input.mouse_delta();
        let rot = Rotor3::from_rotation_xz(dx as f32 * (PI / 4.0) * DT as f32);

        player.trf.prepend_rotation(rot);

        player
            .trf
            .prepend_translation(Vec3::new(move_x * 100.0, 0., move_z * 100.0));

        // find current room
        let current_room = self
            .player
            .map
            .rooms_list
            .get(&self.player.current_room)
            .unwrap();
        let player_x = player.trf.translation.x;
        let player_z = player.trf.translation.z;
        let room_bottom_left_corner_x = current_room.bottom_left_corner[0];
        let room_bottom_left_corner_z = current_room.bottom_left_corner[1];
        // check for any overlaps
        if player_x > room_bottom_left_corner_x + ROOM_WIDTH - 10. {
            player.trf.translation.x = room_bottom_left_corner_x + ROOM_WIDTH - 10.;
        }
        if player_z > room_bottom_left_corner_z + ROOM_LENGTH - 10. {
            player.trf.translation.z = room_bottom_left_corner_z + ROOM_LENGTH - 10.;
        }
        if player_x < room_bottom_left_corner_x + 10. {
            player.trf.translation.x = room_bottom_left_corner_x + 10.;
        }
        if player_z < room_bottom_left_corner_z + 10. {
            player.trf.translation.z = room_bottom_left_corner_z + 10.;
        }
        //use this to only check collisions w walls around the player
        // for wall_idx in self
        //     .player
        //     .map
        //     .rooms_list
        //     .get(&self.player.current_room)
        //     .unwrap()
        //     .flats
        // {
        // for (wall_idx, wall) in self.player.map.walls.iter().enumerate() {
        //     if let Some(r) = &wall.wall {
        //         if let Some(disp) = displacement(player_shape, &r) {
        //             // dbg!(&wall_idx, "displaced");
        //             // println!("\n\n\n\n\n\n displaced \n\n\\n\n\n");
        //             // panic!("displaced");
        //             // dbg!(&disp);
        //             let scaled_disp = disp / player.trf.scale;
        //             // dbg!(&scaled_disp);
        //             player.trf.translation += scaled_disp;
        //         }
        //     }
        // }
        // update player's coordinates accordingly
        // for wall_idx in self
        //     .player
        //     .map
        //     .rooms_list
        //     .get(&self.player.current_room)
        //     .unwrap()
        //     .flats
        // {
        //     //use this to only check collisions w walls around the player
        //     dbg!(&self.player.map.walls[wall_idx].wall.center);

        //     if wall_idx > 0 {
        //         continue;
        //     }

        //     if let Some(disp) = displacement(player_shape, &self.player.map.walls[wall_idx].wall) {
        //         dbg!(&wall_idx, "displaced");
        //         // player.trf.translation -= disp;
        //     }
        //     continue;
        // }

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
            let mut rendered = false;
            for key in self.player.keys_grabbed.iter_mut() {
                if key.opens_wallid.eq(&m_i) {
                    //also change the wall vec to be none for this one
                    self.player.map.walls[m_i].wall = None;
                    rs.render_flat(m.open_model.clone(), m.trf, m_i);
                    rendered = true;
                    break;
                }
            }
            if !rendered {
                rs.render_flat(m.model.clone(), m.trf, m_i);
            }
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

    let key_tex = engine.load_texture(std::path::Path::new("content/silver.png"))?;
    let key_meshes = engine.load_textured(std::path::Path::new("content/silver-key.obj"))?;
    let key = engine.create_textured_model(key_meshes, vec![key_tex]);
    let floor_tex = engine.load_texture(std::path::Path::new("content/marble-floor.png"))?;
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

    let trophy_tex = engine.load_texture(std::path::Path::new("content/gold-trophy.png"))?;
    let trophy_meshes =
        engine.load_textured(std::path::Path::new("content/trophyobjectfile.obj"))?;
    let trophy = engine.create_textured_model(trophy_meshes, vec![trophy_tex, trophy_tex]);
    let trophy_texture = Textured {
        trf: Similarity3::new(Vec3::new(-200., 0.0, 590.), Rotor3::identity(), 5.0),
        model: Rc::clone(&trophy),
    };

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

        let bottom_left_corner = room["bottom_left_corner"].as_array().unwrap();
        let bottom_left_corner_arr: [f32; 2] = [
            bottom_left_corner[0].as_f64().unwrap() as f32,
            bottom_left_corner[1].as_f64().unwrap() as f32,
        ];
        map.add_room(
            room_id,
            flats_arr,
            connected_rooms_arr,
            bottom_left_corner_arr,
        );
    }

    let mut flats_vec: Vec<Flat> = vec![];
    let mut walls_vec: Vec<Wall> = vec![];
    let flats = json.get("flats").unwrap();
    for (i, flat) in flats.as_array().unwrap().iter().enumerate() {
        let mut rot = Rotor3::identity();

        //1.57079 Rust was complaining about this value. now is std::f32::consts::FRAC_PI_2

        let mut rotate = false;
        if !(flat["is_identity"].as_bool().unwrap()) {
            rotate = true;
            rot = Rotor3::from_rotation_xz(std::f32::consts::FRAC_PI_2)
        }

        let x = flat["x"].as_f64().unwrap() as f32;
        let y = -15.0;
        let z = flat["z"].as_f64().unwrap() as f32;

        let mut wall = false;

        let model = {
            if flat["door"].as_i64().unwrap() as i32 == 0 {
                // Wall model without a door
                wall = true;
                wall_no_door_model.clone()
            } else if flat["door"].as_i64().unwrap() as i32 == 1 {
                // Wall model with an open door
                wall_with_door_opened_model.clone()
            } else if flat["door"].as_i64().unwrap() as i32 == 2 {
                // Wall model with a locked door
                wall = true;
                wall_with_door_closed_model.clone()
            } else {
                panic!("Invalid value for specification of wall.")
            }
        };

        let trf = Similarity3::new(Vec3::new(x, y, z), rot, 100.);
        let half_widths_wall = if rotate {
            Vec2::new(ROOM_WIDTH / 2., WALL_THICKNESS / 2.)
        } else {
            Vec2::new(WALL_THICKNESS / 2., ROOM_LENGTH / 2.)
        };

        let disp_mult = if i == 0 || i == 2 { 1.0 } else { -1.0 };

        let mut wall_coll = if wall {
            Some(AABB2D {
                center: Vec2::new(trf.translation.x, trf.translation.z) - FIX,
                half_widths: half_widths_wall,
                disp_mult,
            })
        } else {
            None
        };
        // dbg!(trf);

        let new_wall = Wall { wall: wall_coll };

        let new_flat = Flat {
            trf,
            model: model.clone(),
            open_model: wall_with_door_opened_model.clone(),
        };
        flats_vec.push(new_flat);
        walls_vec.push(new_wall);
    }
    map.add_walls(walls_vec);

    let player_obj = GameObject {
        trf: Similarity3::new(Vec3::new(150.0, -15.0, 0.0), Rotor3::identity(), 0.1),
        model,
        animation,
        state: AnimationState { t: 0.0 },
    };

    let key_rot = Rotor3::from_rotation_yz(std::f32::consts::FRAC_PI_2 * -1.);
    let key_positions = vec![
        Similarity3::new(Vec3::new(0.0, 0.0, -10.0), key_rot, 2.),
        Similarity3::new(Vec3::new(20.0, 0.0, -15.0), key_rot, 2.),
        Similarity3::new(Vec3::new(364.0, 0., 30.0), key_rot, 2.),
        Similarity3::new(Vec3::new(91.0, 0., 287.0), key_rot, 2.),
        Similarity3::new(Vec3::new(378., 0., 254.0), key_rot, 2.),
        Similarity3::new(Vec3::new(110.0, 0., 563.0), key_rot, 2.),
    ];

    let (keys, mut key_textureds) = multiple_key_pairs(
        key_positions,
        key,
        vec![(0, 1), (0, 3), (1, 6), (2, 8), (3, 11), (4, 13)],
    );

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
        trf: Similarity3::new(Vec3::new(0.0, -32.0, 0.0), Rotor3::identity(), 15.0),
        model: floor,
    }]);
    all_textureds.push(trophy_texture);
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

    // load and play background music
    let mut audio_manager = AudioManager::new(AudioManagerSettings::default()).unwrap();

    let sound_handle = audio_manager
        .load_sound(
            "content/background.mp3",
            SoundSettings::new().semantic_duration(Tempo(128.0).beats_to_seconds(8.0)),
        )
        .unwrap();
    let mut arrangement_handle = audio_manager
        .add_arrangement(Arrangement::new_loop(
            &sound_handle,
            LoopArrangementSettings::default(),
        ))
        .unwrap();
    arrangement_handle
        .play(InstanceSettings::default())
        .unwrap();

    engine.play(world)
}
