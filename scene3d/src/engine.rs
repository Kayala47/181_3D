use crate::camera::Camera;
use crate::input::Input;
use crate::renderer::textured::TexturedMeshRenderer;
use crate::types::*;
use crate::vulkan::Vulkan;
use color_eyre::eyre::{ensure, eyre, Result};
use std::collections::HashMap;
use vulkano::sync::GpuFuture;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

// TODO: figure out actual sizes
const GRAB_THRESHOLD: f32 = 10.0;
const ROOM_RADIUS: f32 = 50.0; //not the right word, but half the length.
const DOOR_THRESHOLD: f32 = 5.0; // if within this distance of a door, need to have a key

const WALL_X: f32 = 0.1 * 100.0;
const WALL_Y: f32 = 1.0 * 100.0;
const WALL_Z: f32 = 0.5 * 100.0;

const DOOR_X: f32 = 0.1 * 100.0;
const DOOR_Y: f32 = 0.33 * 100.0;
const DOOR_Z: f32 = 0.25 * 100.0;

pub struct WindowSettings {
    pub w: usize,
    pub h: usize,
    pub title: String,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            w: 1024,
            h: 768,
            title: "Engine Window".to_string(),
        }
    }
}

pub struct Engine {
    textures: HashMap<TextureRef, Texture>,
    next_texture: usize,
    meshes: HashMap<MeshRef, Mesh>,
    next_mesh: usize,
    event_loop: Option<EventLoop<()>>,
    camera: Camera,
    objects: Vec<GameObject>,
    vulkan: Vulkan,
    input: Input,
    tex_mesh_renderer: TexturedMeshRenderer,
}

impl Engine {
    pub fn new(ws: WindowSettings) -> Self {
        let event_loop = EventLoop::new();
        let wb = WindowBuilder::new()
            .with_inner_size(winit::dpi::LogicalSize::new(ws.w as f32, ws.h as f32))
            .with_title(ws.title);
        let input = Input::new();
        let mut vulkan = Vulkan::new(wb, &event_loop);
        Self {
            tex_mesh_renderer: TexturedMeshRenderer::new(&mut vulkan),
            vulkan,
            event_loop: Some(event_loop),
            camera: Camera::look_at(Vec3::new(0., 0., 0.), Vec3::new(0., 0., 1.), Vec3::unit_y()),
            objects: vec![],
            input,
            next_texture: 0,
            next_mesh: 0,
            textures: HashMap::new(),
            meshes: HashMap::new(),
        }
    }
    pub fn set_camera(&mut self, cam: Camera) {
        self.camera = cam;
    }

    pub fn get_camera(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn get_input(&mut self) -> &Input {
        &self.input
    }

    pub fn create_game_object(&mut self, model: Option<&Model>, trf: Isometry3) -> &mut GameObject {
        self.objects.push(GameObject {
            model: model.cloned(),
            transform: trf,
        });
        self.objects.last_mut().unwrap()
    }
    pub fn objects_mut(&mut self) -> impl Iterator<Item = &mut GameObject> {
        self.objects.iter_mut()
    }
    pub fn load_texture(&mut self, path: &std::path::Path) -> Result<TextureRef> {
        let img = Image::from_file(path)?;
        let tid = self.next_texture;
        self.next_texture += 1;
        let (vulk_img, fut) = ImmutableImage::from_iter(
            img.as_slice().iter().copied(),
            vulkano::image::ImageDimensions::Dim2d {
                width: img.sz.x,
                height: img.sz.y,
                array_layers: 1,
            },
            vulkano::image::MipmapsCount::One,
            vulkano::format::Format::R8G8B8A8_SRGB,
            self.vulkan.queue.clone(),
        )?;
        // fancy!
        let old_fut = self.vulkan.previous_frame_end.take();
        self.vulkan.previous_frame_end = match old_fut {
            None => Some(Box::new(fut)),
            Some(old_fut) => Some(Box::new(old_fut.join(fut))),
        };
        self.textures.insert(
            TextureRef(tid),
            Texture {
                image: img,
                texture: vulk_img,
            },
        );
        Ok(TextureRef(tid))
    }
    pub fn load_mesh(&mut self, path: &std::path::Path, scale: f32) -> Result<MeshRef> {
        let mid = self.next_mesh;
        self.next_mesh += 1;

        use russimp::scene::{PostProcess, Scene};
        let mut scene = Scene::from_file(
            path.to_str()
                .ok_or_else(|| eyre!("Mesh path can't be converted to string: {:?}", path))?,
            vec![
                PostProcess::Triangulate,
                PostProcess::JoinIdenticalVertices,
                PostProcess::FlipUVs,
            ],
        )?;
        let mesh = scene.meshes.swap_remove(0);
        let verts = &mesh.vertices;
        let uvs = mesh
            .texture_coords
            .first()
            .ok_or_else(|| eyre!("Mesh fbx has no texture coords: {:?}", path))?
            .as_ref();
        let uvs =
            uvs.ok_or_else(|| eyre!("Mesh fbx doesn't specify texture coords: {:?}", path))?;
        ensure!(
            mesh.faces[0].0.len() == 3,
            "Mesh face has too many indices: {:?}",
            mesh.faces[0]
        );
        // This is safe to allow because we need an ExactSizeIterator of faces
        #[allow(clippy::needless_collect)]
        let faces: Vec<u32> = mesh
            .faces
            .iter()
            .flat_map(|v| v.0.iter().copied())
            .collect();
        let (vb, vb_fut) = vulkano::buffer::ImmutableBuffer::from_iter(
            verts.iter().zip(uvs.iter()).map(|(pos, uv)| VertexUV {
                position: [pos.x * scale, pos.y * scale, pos.z * scale],
                uv: [uv.x, uv.y],
            }),
            vulkano::buffer::BufferUsage::vertex_buffer(),
            self.vulkan.queue.clone(),
        )?;
        let (ib, ib_fut) = vulkano::buffer::ImmutableBuffer::from_iter(
            faces.into_iter(),
            vulkano::buffer::BufferUsage::index_buffer(),
            self.vulkan.queue.clone(),
        )?;
        let load_fut = vb_fut.join(ib_fut);
        let old_fut = self.vulkan.previous_frame_end.take();
        self.vulkan.previous_frame_end = match old_fut {
            None => Some(Box::new(load_fut)),
            Some(old_fut) => Some(Box::new(old_fut.join(load_fut))),
        };
        self.meshes.insert(
            MeshRef(mid),
            Mesh {
                mesh,
                verts: vb,
                idx: ib,
            },
        );
        Ok(MeshRef(mid))
    }
    pub fn create_model(&self, mesh: &MeshRef, texture: &TextureRef) -> Model {
        Model {
            mesh: *mesh,
            texture: *texture,
        }
    }
    pub fn play(mut self, f: impl Fn(&mut Self) + 'static) -> Result<()> {
        let ev = self.event_loop.take().unwrap();
        ev.run(move |event, _, control_flow| {
            match event {
                // Nested match patterns are pretty useful---see if you can figure out what's going on in this match.
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(_),
                    ..
                } => {
                    self.vulkan.recreate_swapchain = true;
                }
                // NewEvents: Let's start processing events.
                Event::NewEvents(_) => {}
                // WindowEvent->KeyboardInput: Keyboard input!
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            input: in_event, ..
                        },
                    ..
                } => {
                    self.input.handle_key_event(in_event);
                }
                Event::MainEventsCleared => {
                    // track DT, accumulator, ...
                    {
                        f(&mut self);
                        self.input.next_frame();
                    }
                    self.render3d();
                }
                _ => (),
            }
        });
    }
    fn render3d(&mut self) {
        use vulkano::command_buffer::{
            AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents,
        };

        let vulkan = &mut self.vulkan;
        vulkan.recreate_swapchain_if_necessary();
        let image_num = vulkan.get_next_image();
        if image_num.is_none() {
            return;
        }
        let image_num = image_num.unwrap();
        let mut builder = AutoCommandBufferBuilder::primary(
            vulkan.device.clone(),
            vulkan.queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        for obj in self.objects.iter() {
            if let Some(model) = obj.model {
                let mesh = &self.meshes[&model.mesh];
                let tex = &self.textures[&model.texture];
                self.tex_mesh_renderer
                    .push_model(model, mesh, tex, obj.transform);
            }
        }
        self.tex_mesh_renderer.prepare_draw(&self.camera);

        builder
            .begin_render_pass(
                vulkan.framebuffers[image_num].clone(),
                SubpassContents::Inline,
                vec![[0.0, 0.0, 0.0, 0.0].into(), (1.0).into()],
            )
            .unwrap()
            .set_viewport(0, [vulkan.viewport.clone()]);

        self.tex_mesh_renderer.draw(&mut builder);

        builder.end_render_pass().unwrap();

        let command_buffer = builder.build().unwrap();
        vulkan.execute_commands(command_buffer, image_num);
    }
}

pub struct Room {
    id: usize,
    gameobject: GameObject,
    objects: Vec<Key>,
    doors: [bool; 4],            //N, E, S, W yes/no for doors
    connected_rooms: [usize; 4], //point by ID
}

impl Room {
    pub fn move_by(&mut self, vec: Vec3) {
        self.gameobject.move_by(vec);
    }

    pub fn get_key(&mut self, idx: usize) -> Key {
        self.objects.swap_remove(idx)
    }
}

pub struct Key {
    roomid: usize, //the room they open
    gameobject: GameObject,
    picked_up: bool,
}

impl Key {
    pub fn move_by(&mut self, vec: Vec3) {
        self.gameobject.move_by(vec);
    }

    pub fn pick_up(mut self, player: &mut Player) {
        self.picked_up = true;
        player.keys_grabbed.push(self);
    }
}

pub struct World {
    start_room: Room,
    rooms_list: HashMap<usize, Room>,
    end_room: Room,
}

pub struct Player {
    object: GameObject,
    keys_grabbed: Vec<Key>,
    current_room: usize, //id of room
    world: World,        //so the player knows about the rooms
}

impl Player {
    pub fn grab(&mut self, world: &mut World) {
        //checks if keys are nearby and grabs them

        let curr_pos = self.object.transform.translation;

        //two steps: filter out the ones that match first, then actually pick them up

        let keys = &mut world
            .rooms_list
            .get_mut(&self.current_room)
            .unwrap()
            .objects;

        if let Some(pos) = keys
            .iter()
            .position(|k| distance(curr_pos, k.gameobject.transform.translation) < GRAB_THRESHOLD)
        {
            //check distance
            let key = keys.swap_remove(pos);
            key.pick_up(self);
        }
    }
}

fn distance(v1: Vec3, v2: Vec3) -> f32 {
    (v1 - v2).mag()
}

pub struct GameState {
    player: Player,
}

pub struct GameObject {
    model: Option<Model>,
    transform: Isometry3, //has a translation and rotation
}
impl GameObject {
    pub fn move_by(&mut self, vec: Vec3) {
        self.transform.append_translation(vec);
    }
}
use crate::image::Image;
use std::sync::Arc;
use vulkano::buffer::ImmutableBuffer;
use vulkano::image::immutable::ImmutableImage;

use bytemuck::{Pod, Zeroable};
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod, Zeroable)]
pub struct VertexUV {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}
vulkano::impl_vertex!(VertexUV, position, uv);

pub struct Mesh {
    pub mesh: russimp::mesh::Mesh,
    pub verts: Arc<ImmutableBuffer<[VertexUV]>>,
    pub idx: Arc<ImmutableBuffer<[u32]>>,
}
pub struct Texture {
    pub image: Image,
    pub texture: Arc<ImmutableImage>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Model {
    pub mesh: MeshRef,
    pub texture: TextureRef,
}

// string_interner
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshRef(usize);
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureRef(usize);
