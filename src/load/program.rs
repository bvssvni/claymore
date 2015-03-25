use std::old_io as io;
use std::old_path::Path;
use gfx;
use gfx::traits::*;

#[derive(Clone)]
#[shader_param]
pub struct Params<R: gfx::Resources> {
    #[name = "u_Transform"]
    pub mvp: [[f32; 4]; 4],
    #[name = "u_NormalRotation"]
    pub normal: [[f32; 3]; 3],
    #[name = "u_Color"]
    pub color: [f32; 4],
    #[name = "t_Diffuse"]
    pub texture: gfx::shade::TextureParam<R>,
}

#[derive(Clone, Debug)]
pub enum Error {
    Read(Path, io::IoError),
    Create(gfx::ProgramError),
}

pub fn load<R: gfx::Resources, F: gfx::Factory<R>>(name: &str, factory: &mut F)
    -> Result<gfx::ProgramHandle<R>, Error> {
    use std::old_io::Reader;
    let src_vert = {
        let path = Path::new(format!("shader/{}.glslv", name));
        match io::File::open(&path).read_to_end() {
            Ok(c) => c,
            Err(e) => return Err(Error::Read(path, e)),
        }
    };
    let src_frag = {
        let path = Path::new(format!("shader/{}.glslf", name));
        match io::File::open(&path).read_to_end() {
            Ok(c) => c,
            Err(e) => return Err(Error::Read(path, e)),
        }
    };
    factory.link_program(src_vert.as_slice(), src_frag.as_slice())
           .map_err(|e| Error::Create(e))
}
