use crate::{canvas::Color, integrator::Integrator, ray::Ray, render::Config, scene::Scene};

/// This is an integrator that will color an image by the number of steps it takes to reach a point
/// in the scene. Running out of fuel or escapign the bounds of the scene will be reported as
/// black.
///
/// While anti-aliasing is supported, it's not really helpful in this context as it will give the
/// average number of steps for a given pixel.
pub struct DebugSteps;

impl DebugSteps {
    pub fn new() -> Self {
        DebugSteps
    }
}

impl Integrator for DebugSteps {
    fn render(&self, cfg: &Config, scene: &Scene, ray: &Ray) -> Color {
        let step_max = cfg.max_steps as f32;
        if let Some(res) = ray.march(cfg.max_steps, |pt| scene.sdf(pt)) {
            let step_val = 1.0 - (res.steps as f32) / step_max;
            Color::new(step_val, 0.0, step_val)
        } else {
            Color::black()
        }
    }
}
