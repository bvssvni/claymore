extern crate gfx;
extern crate gfx_pipeline;
extern crate claymore_scene as scene;
extern crate claymore_load as load;

use gfx_pipeline::forward::Pipeline;


pub struct App<D: gfx::Device> {
    frame: gfx::Frame<D::Resources>,
    scene: scene::Scene<D::Resources, load::Scalar>,
    pipeline: Pipeline<D>,
}

impl<
    R: gfx::Resources,
    C: gfx::CommandBuffer<R>,
    D: gfx::Device<Resources = R, CommandBuffer = C> + gfx::Factory<R>
> App<D> {
    pub fn new(device: &mut D, width: u16, height: u16) -> App<D>
    {
        // load the scene
        let (scene, texture) = {
            let mut context = load::Context::new(device).unwrap();
            let mut scene = load::scene("data/vika", &mut context).unwrap();
            scene.cameras[0].projection.aspect = width as f32 / height as f32;
            (scene, (context.texture_black.clone(), None))
        };
        // create the pipeline
        let mut pipeline = Pipeline::new(device, texture).unwrap();
        pipeline.background = Some([0.2, 0.3, 0.4, 1.0]);
        // done
        App {
            frame: gfx::Frame::new(width, height),
            scene: scene,
            pipeline: pipeline,
        }
    }

    pub fn render(&mut self) -> Result<gfx::SubmitInfo<D>, gfx_pipeline::Error> {
        use gfx_pipeline::Pipeline;
        self.scene.world.update();
        let camera = self.scene.cameras[0].clone();
        self.pipeline.render(&self.scene, &camera, &self.frame)
    }
}
