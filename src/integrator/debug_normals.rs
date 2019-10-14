use crate::{canvas::Color, integrator::Integrator, ray::Ray, render::Config, scene::Scene};

pub struct DebugNormals;

impl DebugNormals {
    pub fn new() -> Self {
        DebugNormals
    }
}

impl Integrator for DebugNormals {
    fn render(&self, cfg: &Config, scene: &Scene, ray: &Ray) -> Color {
        if let Some(res) = ray.march(cfg.max_steps, |pt| scene.sdf(pt)) {
            let normal = res.normal(|pt| scene.sdf(pt));
            Color::new(
                0.5 + normal.x / 2.0,
                0.5 + normal.y / 2.0,
                0.5 + normal.z / 2.0,
            )
        } else {
            Color::black()
        }
    }
}
