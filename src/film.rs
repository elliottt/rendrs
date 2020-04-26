use nalgebra::Point2;

use crate::bounds::Bounds2;
use crate::filter::Filter;
use crate::float::Float;

pub type Resolution = Point2<u64>;

pub struct Film {
    pub res: Resolution,
    pub crop: Bounds2<Float>,
    pub filter: Box<dyn Filter>,
    pub cropped_bounds: Bounds2<u64>,
}

impl Film {
    pub fn new(res: Resolution, crop: Bounds2<Float>, filter: Box<dyn Filter>) -> Self {
        let cropped_bounds = {
            let min = Point2::new(
                (res.x as Float * crop.min.x).ceil() as u64,
                (res.y as Float * crop.min.y).ceil() as u64,
            );

            let max = Point2::new(
                (res.x as Float * crop.max.x).ceil() as u64,
                (res.y as Float * crop.max.y).ceil() as u64,
            );

            Bounds2{ min, max }
        };

        Film { res, crop, filter, cropped_bounds }
    }

    pub fn cropped_bounds(&self) -> &Bounds2<u64> {
        &self.cropped_bounds
    }
}
