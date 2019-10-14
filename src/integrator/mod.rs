use crate::{canvas::Color, ray::Ray, render::Config, scene::Scene};

pub mod debug_normals;
pub mod debug_steps;
pub mod whitted;

pub use debug_normals::DebugNormals;
pub use debug_steps::DebugSteps;
pub use whitted::Whitted;

pub trait Integrator: Send + Sync {
    /// Render the given scene using this integrator.
    fn render(&self, cfg: &Config, scene: &Scene, ray: &Ray) -> Color;
}
