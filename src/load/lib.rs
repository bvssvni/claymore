#![feature(collections, convert, unsafe_destructor)]

#[macro_use]
extern crate log;
extern crate rustc_serialize;
extern crate cgmath;
extern crate gfx;
extern crate gfx_texture;
extern crate gfx_scene;
extern crate claymore_scene;

mod aux;
pub mod chunk;
mod mesh;
mod mat;
mod program;
mod reflect;
mod scene;

use std::collections::hash_map::{HashMap, Entry};
use std::io;
use std::fs::File;
use rustc_serialize::json;

pub use self::scene::Scalar;


pub static PREFIX_ATTRIB : &'static str = "a_";
pub static PREFIX_UNIFORM: &'static str = "u_";
pub static PREFIX_TEXTURE: &'static str = "t_";

pub type TextureError = String;

pub struct Cache<R: gfx::Resources> {
    meshes: HashMap<String, mesh::Success<R>>,
    textures: HashMap<String, Result<gfx::TextureHandle<R>, TextureError>>,
    programs: HashMap<String, Result<gfx::ProgramHandle<R>, program::Error>>,
}

impl<R: gfx::Resources> Cache<R> {
    pub fn new() -> Cache<R> {
        Cache {
            meshes: HashMap::new(),
            textures: HashMap::new(),
            programs: HashMap::new(),
        }
    }
}

pub struct Context<'a, R: 'a + gfx::Resources, F: 'a + gfx::Factory<R>> {
    pub cache: Cache<R>,
    pub factory: &'a mut F,
    pub prefix: String,
    pub texture_black: gfx::TextureHandle<R>,
    pub sampler_point: gfx::SamplerHandle<R>,
}

#[derive(Clone, Debug)]
pub enum ContextError {
    Texture(gfx::tex::TextureError),
    Program(gfx::ProgramError),
}

impl<'a, R: gfx::Resources, F: gfx::Factory<R>> Context<'a, R, F> {
    pub fn new(factory: &'a mut F) -> Result<Context<'a, R, F>, ContextError> {
        let tinfo = gfx::tex::TextureInfo {
            width: 1,
            height: 1,
            depth: 1,
            levels: 1,
            format: gfx::tex::RGBA8,
            kind: gfx::tex::TextureKind::Texture2D,
        };
        let image_info = tinfo.to_image_info();
        let texture = match factory.create_texture(tinfo) {
            Ok(t) => match factory.update_texture(&t, &image_info, &[0u8, 0, 0, 0]) {
                Ok(()) => t,
                Err(e) => return Err(ContextError::Texture(e)),
            },
            Err(e) => return Err(ContextError::Texture(e)),
        };
        let sampler = factory.create_sampler(gfx::tex::SamplerInfo::new(
            gfx::tex::FilterMethod::Scale,
            gfx::tex::WrapMode::Tile
        ));
        Ok(Context {
            cache: Cache::new(),
            factory: factory,
            prefix: String::new(),
            texture_black: texture,
            sampler_point: sampler,
        })
    }

    fn read_mesh_collection(&mut self, path_str: &str) -> Result<(), mesh::Error> {
        info!("Loading mesh collection from {}", path_str);
        let path = format!("{}/{}.k3mesh", self.prefix, path_str);
        match File::open(path) {
            Ok(file) => {
                let size = file.metadata().unwrap().len() as u32;
                let mut reader = chunk::Root::new(path_str.to_string(), file);
                while reader.get_pos() < size {
                    let (name, success) = try!(mesh::load(&mut reader, self.factory));
                    let full_name = format!("{}@{}", name, path_str);
                    self.cache.meshes.insert(full_name, success);
                }
                Ok(())
            },
            Err(e) => Err(mesh::Error::Path(e)),
        }
    }

    pub fn request_mesh(&mut self, path: &str)
                        -> Result<mesh::Success<R>, mesh::Error> {
        match self.cache.meshes.get(path) {
            Some(m) => return Ok(m.clone()),
            None => (),
        }
        let mut split = path.split('@');
        split.next().unwrap();  //skip name
        match split.next() {
            Some(container) => {
                try!(self.read_mesh_collection(container));
                Ok(self.cache.meshes[path].clone())
            },
            None => Err(mesh::Error::Other),
        }
    }

    pub fn request_texture(&mut self, path_str: &str)
                           -> Result<gfx::TextureHandle<R>, TextureError> {
        match self.cache.textures.entry(path_str.to_string()) {
            Entry::Occupied(v) => v.get().clone(),
            Entry::Vacant(v) => {
                info!("Loading texture from {}", path_str);
                let path_str = format!("{}{}", self.prefix, path_str);
                let tex_maybe = gfx_texture::Texture::from_path(self.factory, path_str.as_ref())
                    .map(|t| t.handle);
                v.insert(tex_maybe).clone()
            },
        }
    }

    pub fn request_program(&mut self, name: &str)
                           -> Result<gfx::ProgramHandle<R>, program::Error> {
        match self.cache.programs.entry(name.to_string()) {
            Entry::Occupied(v) => v.get().clone(),
            Entry::Vacant(v) => {
                info!("Loading program {}", name);
                let prog_maybe = program::load(name, self.factory);
                v.insert(prog_maybe).clone()
            },
        }
    }
}

#[derive(Debug)]
pub enum SceneError {
    Open(io::Error),
    Read(io::Error),
    Decode(json::DecoderError),
    Parse(scene::Error),
}

pub fn scene<'a, R: gfx::Resources, F: gfx::Factory<R>>(path_str: &str,
             context: &mut Context<'a, R, F>)
             -> Result<claymore_scene::Scene<R, scene::Scalar>, SceneError> {
    use std::io::Read;
    info!("Loading scene from {}", path_str);
    context.prefix = path_str.to_string();
    let path = format!("{}.json", path_str);
    match File::open(&path) {
        Ok(mut file) => {
            let mut s = String::new();
            match file.read_to_string(&mut s) {
                Ok(_) => match json::decode(&s) {
                    Ok(raw) => match scene::load(raw, context) {
                        Ok(s) => Ok(s),
                        Err(e) => Err(SceneError::Parse(e)),
                    },
                    Err(e) => Err(SceneError::Decode(e)),
                },
                Err(e) => Err(SceneError::Read(e)),
            }
        },
        Err(e) => Err(SceneError::Open(e)),
    }
}

pub fn mesh<'a, R: gfx::Resources, F: gfx::Factory<R>>(path_str: &str, factory: &mut F)
            -> Result<(String, mesh::Success<R>), mesh::Error> {
    info!("Loading mesh from {}", path_str);
    let path = format!("{}.k3mesh", path_str);
    match File::open(&path) {
        Ok(file) => {
            let mut reader = chunk::Root::new(path, file);
            mesh::load(&mut reader, factory)
        },
        Err(e) => Err(mesh::Error::Path(e)),
    }
}
