use crate::{
    scene::Scene,
    shapes::{PrimShape, Shape, ShapeId},
};
use failure::{format_err, Error};
use nalgebra::Point3;
use std::io::BufRead;

#[derive(Debug)]
pub struct Obj {
    vertices: Vec<Point3<f32>>,
    faces: Vec<[usize; 3]>,
}

impl Obj {
    pub fn new() -> Self {
        Obj {
            vertices: Vec::new(),
            faces: Vec::new(),
        }
    }

    pub fn push_vertex(&mut self, point: Point3<f32>) {
        self.vertices.push(point)
    }

    pub fn push_face(&mut self, face: [usize; 3]) {
        self.faces.push(face)
    }

    pub fn parse<In>(input: In) -> Result<Self, Error>
    where
        In: BufRead,
    {
        let mut obj = Obj::new();

        for line in input.lines() {
            let line = line?;

            let mut chunks = line.split_whitespace();

            match chunks.next() {
                Some("v") => {
                    let a = chunks
                        .next()
                        .ok_or_else(|| format_err!("bad vertex"))?
                        .parse()?;
                    let b = chunks
                        .next()
                        .ok_or_else(|| format_err!("bad vertex"))?
                        .parse()?;
                    let c = chunks
                        .next()
                        .ok_or_else(|| format_err!("bad vertex"))?
                        .parse()?;
                    obj.push_vertex(Point3::new(a, b, c));
                }

                Some("f") => {
                    let a = chunks
                        .next()
                        .ok_or_else(|| format_err!("bad face"))?
                        .parse()?;
                    let b = chunks
                        .next()
                        .ok_or_else(|| format_err!("bad face"))?
                        .parse()?;
                    let c = chunks
                        .next()
                        .ok_or_else(|| format_err!("bad face"))?
                        .parse()?;
                    obj.push_face([a, b, c]);
                }

                _ => (),
            }
        }

        Ok(obj)
    }

    /// Add the object to a scene.
    pub fn add_to_scene(&self, scene: &mut Scene) -> Result<ShapeId, Error> {
        let mut triangles = Vec::with_capacity(self.faces.len());

        for face in self.faces.iter() {
            let a = self.vertices.get(face[0] - 1).expect("face out of bounds");
            let b = self.vertices.get(face[1] - 1).expect("face out of bounds");
            let c = self.vertices.get(face[2] - 1).expect("face out of bounds");
            let shape = PrimShape::triangle(a, b, c);
            triangles.push(scene.add(Shape::PrimShape { shape }));
        }

        Ok(scene.add(Shape::group(scene, triangles)))
    }
}
