
use failure::{Error,format_err};
use serde_yaml::Value;
use nalgebra::{Point3,Vector3,Matrix4};

use std::{
    collections::BTreeMap,
    fs,
    path::Path,
};

use crate::{
    canvas::Color,
    camera::Camera,
    material::{Light,Material,MaterialId},
    pattern::{Pattern,PatternId},
    scene::Scene,
    shapes::{ShapeId,Shape,PrimShape},
};


pub fn parse<P>(path: P)
    -> Result<(Scene,Vec<Camera>), Error>
    where P: AsRef<Path>
{
    let bytes = String::from_utf8(fs::read(path)?)?;
    let parsed: Value = serde_yaml::from_str(bytes.as_str())?;

    let ctx = Context::new(&parsed);
    let scene = parse_scene(&ctx)?;
    let cameras = parse_cameras(&ctx)?;

    Ok((scene,cameras))
}

fn optional<A,Err>(res: Result<A,Err>) -> Option<A> {
    if let Ok(a) = res {
        Some(a)
    } else {
        None
    }
}

/// Parser context.
struct Context<'a> {
    focus: &'a Value,
}

impl<'a> Context<'a> {
    fn new<'b>(focus: &'b Value) -> Context<'b> {
        Context { focus }
    }

    fn get_field(&self, label: &str) -> Result<Context<'a>,Error> {
        self.focus.get(label).map_or_else(
            || Err(format_err!("missing field `{}`", label)),
            |val| Ok(Context::new(val)))
    }

    fn get_at(&self, index: usize) -> Result<Context<'a>,Error> {
        self.focus.get(index).map_or_else(
            || Err(format_err!("missing sequence entry `{}`", index)),
            |val| Ok(Context::new(val)))
    }

    fn as_sequence(&self) -> Result<impl Iterator<Item=Context<'a>>,Error> {
        let elems = self.focus.as_sequence().map_or_else(
            || Err(format_err!("expected a sequence")),
            |elems| Ok(elems))?;
        Ok(elems.iter().map(|elem| Context::new(elem)))
    }

    fn as_f32(&self) -> Result<f32,Error> {
        self.focus.as_f64().map_or_else(
            || Err(format_err!("expected a float")),
            |val| Ok(val as f32))
    }

    fn as_usize(&self) -> Result<usize,Error> {
        self.focus.as_u64().map_or_else(
            || Err(format_err!("expected an integer")),
            |val| Ok(val as usize))
    }

    fn as_str(&self) -> Result<String,Error> {
        self.focus.as_str().map_or_else(
            || Err(format_err!("expected a string")),
            |val| Ok(val.to_string()))
    }
}

fn parse_scene(ctx: &Context) -> Result<Scene,Error> {
    let mut scene = Scene::new();

    let pats = optional(ctx.get_field("patterns")).map_or_else(
        || Ok(BTreeMap::new()),
        |ctx| parse_pats(&ctx, &mut scene))?;

    let mats = optional(ctx.get_field("materials")).map_or_else(
        || Ok(BTreeMap::new()),
        |ctx| parse_mats(&ctx, &mut scene))?;

    let objs = optional(ctx.get_field("objects")).map_or_else(
        || Ok(BTreeMap::new()),
        |ctx| parse_objs(&ctx, &mut scene, &pats, &mats))?;

    parse_lights(&ctx.get_field("lights")?, &mut scene)?;

    parse_roots(&ctx.get_field("scene")?, &mut scene, objs)?;

    Ok(scene)
}

fn parse_cameras(ctx: &Context) -> Result<Vec<Camera>,Error> {

    let entries = ctx.get_field("cameras")?.as_sequence()?;

    let mut cameras = match entries.size_hint() {
        (_, Some(upper)) => Vec::with_capacity(upper),
        _ => Vec::new(),
    };
    for entry in entries {
        let camera = parse_camera(&entry)?;
        cameras.push(camera);
    }

    Ok(cameras)
}

fn parse_camera(ctx: &Context) -> Result<Camera,Error> {
    if let Ok(val) = ctx.get_field("perspective") {
        parse_perspective(&val)
    } else {
        Err(format_err!("unknown camera type"))
    }
}

fn parse_perspective(ctx: &Context) -> Result<Camera,Error> {
    let width = ctx.get_field("width")?.as_usize()?;
    let height = ctx.get_field("height")?.as_usize()?;
    let fov = ctx.get_field("fov")?.as_f32()?;
    let position = parse_point3(&ctx.get_field("position")?)?;
    let target = parse_point3(&ctx.get_field("target")?)?;

    let mut camera = Camera::new(width, height, fov);
    camera.set_transform(Matrix4::look_at_lh(&position, &target, &Vector3::new(0.0, 1.0, 0.0)));

    Ok(camera)
}

#[derive(Debug)]
enum ParsedPat {
    Solid(Color),
    Striped(String,String),
}

fn parse_pats(ctx: &Context, scene: &mut Scene)
    -> Result<BTreeMap<String,PatternId>, Error>
{

    let entries = ctx.as_sequence()?;

    let mut pat_map = BTreeMap::new();

    // build the work queue
    let mut work = match entries.size_hint() {
        (_, Some(upper)) => Vec::with_capacity(upper),
        _ => Vec::new(),
    };
    for entry in entries {
        let name = entry.get_field("name")?.as_str()?;
        let parsed = parse_pat(&entry)?;
        match parsed {
            ParsedPat::Solid(color) => {
                let pid = scene.add_pattern(Pattern::Solid{ color });
                pat_map.insert(name.to_string(), pid);
            },

            _ => {
                work.push((name,parsed));
            },
        }
    }

    let mut next = Vec::with_capacity(work.len());
    while !work.is_empty() {
        let mut progress = false;

        while let Some((name,parsed)) = work.pop() {
            match parsed {
                ParsedPat::Striped(ref a, ref b) => {
                    if let Some(first) = pat_map.get(a) {
                        if let Some(second) = pat_map.get(b) {
                            let pid = scene.add_pattern(Pattern::Stripe{
                                first: *first,
                                second: *second
                            });
                            pat_map.insert(name.to_string(), pid);
                            progress = true;
                            continue;
                        }
                    }
                },

                _ => {},
            }

            next.push((name,parsed));
        }

        std::mem::swap(&mut next, &mut work);

        if !progress {
            return Err(format_err!("invalid patterns: naming cycle, or missing named pattern"));
        }
    }

    Ok(pat_map)
}

fn parse_pat(ctx: &Context) -> Result<ParsedPat,Error> {
    if let Ok(ctx) = ctx.get_field("solid") {
        let color = parse_color(&ctx)?;
        Ok(ParsedPat::Solid(color))
    } else if let Ok(ctx) = ctx.get_field("striped") {
        let a = ctx.get_at(0)?.as_str()?;
        let b = ctx.get_at(1)?.as_str()?;
        Ok(ParsedPat::Striped(a,b))
    } else {
        Err(format_err!("Unknown pattern type"))
    }
}

fn parse_mats(ctx: &Context, scene: &mut Scene) -> Result<BTreeMap<String,MaterialId>,Error> {
    let entries = ctx.as_sequence()?;

    let mut mat_map = BTreeMap::new();

    for entry in entries {
        let name = entry.get_field("name")?.as_str()?;
        let mid = parse_mat(&entry, scene)?;
        mat_map.insert(name, mid);
    }

    Ok(mat_map)
}

fn parse_mat(ctx: &Context, scene: &mut Scene) -> Result<MaterialId,Error> {
    let ambient = ctx.get_field("ambient")?.as_f32()?;
    let diffuse = ctx.get_field("diffuse")?.as_f32()?;
    let specular = ctx.get_field("specular")?.as_f32()?;
    let shininess = ctx.get_field("shininess")?.as_f32()?;
    Ok(scene.add_material(Material{ ambient, diffuse, specular, shininess }))
}

fn parse_objs(
    ctx: &Context,
    scene: &mut Scene,
    pats: &BTreeMap<String,PatternId>,
    mats: &BTreeMap<String,MaterialId>,
) -> Result<BTreeMap<String,ShapeId>,Error> {
    let entries = ctx.as_sequence()?;

    let mut obj_map = BTreeMap::new();

    // build up work queue
    let mut work = match entries.size_hint() {
        (_, Some(upper)) => Vec::with_capacity(upper),
        _ => Vec::new(),
    };

    for entry in entries {
        let name = entry.get_field("name")?.as_str()?;
        let obj = parse_obj(&entry)?;
        work.push((name, obj));
    }

    let mut next = Vec::with_capacity(work.len());
    while !work.is_empty() {
        let mut progress = false;

        while let Some((name,parsed)) = work.pop() {
            match parsed {
                ParsedObj::Sphere => {
                    let sid = scene.add(Shape::PrimShape{ shape: PrimShape::Sphere });
                    obj_map.insert(name,sid);
                    progress = true;
                    continue;
                },

                ParsedObj::Plane => {
                    let sid = scene.add(Shape::PrimShape{ shape: PrimShape::XZPlane });
                    obj_map.insert(name,sid);
                    progress = true;
                    continue;
                },

                ParsedObj::Material(ref pattern,ref material,ref object) => {
                    if let Some(oid) = obj_map.get(object) {
                        let pid = match pattern {
                            None => Ok(scene.default_pattern),
                            Some(ref name) =>
                                pats.get(name).map_or_else(
                                    || Err(format_err!("missing pattern `{}`", name)),
                                    |val| Ok(*val)),
                        }?;
                        let mid = match material {
                            None => Ok(scene.default_material),
                            Some(ref name) =>
                                mats.get(name).map_or_else(
                                    || Err(format_err!("missing material `{}`", name)),
                                    |val| Ok(*val)),
                        }?;
                        let sid = scene.add(Shape::Material{
                            material: mid,
                            pattern: pid,
                            node: *oid,
                        });
                        obj_map.insert(name,sid);
                        progress = true;
                        continue;
                    }
                },

                _ => {},
            }

            next.push((name,parsed));
        }

        if !progress {
            return Err(format_err!("invalid objects: naming cycle, or missing named object"));
        }

        std::mem::swap(&mut next, &mut work);
    }

    Ok(obj_map)
}

enum ParsedObj {
    Sphere,
    Plane,
    Material(Option<String>,Option<String>,String),
}

fn parse_obj(ctx: &Context) -> Result<ParsedObj,Error> {
    if let Ok(_) = ctx.get_field("sphere") {
        Ok(ParsedObj::Sphere)
    } else if let Ok(_) = ctx.get_field("plane") {
        Ok(ParsedObj::Plane)
    } else if let Ok(args) = ctx.get_field("material") {
        let pattern = optional(args.get_field("pattern")).map_or_else(
            || Ok(None), |ctx| ctx.as_str().map(Some))?;
        let material = optional(args.get_field("material")).map_or_else(
            || Ok(None), |ctx| ctx.as_str().map(Some))?;
        let name = args.get_field("object")?.as_str()?;
        Ok(ParsedObj::Material(pattern,material,name))
    } else {
        Err(format_err!("Unknown shape type"))
    }
}

fn parse_lights(ctx: &Context, scene: &mut Scene) -> Result<(),Error> {
    let lights = ctx.as_sequence()?;
    for light in lights {
        parse_light(&light, scene)?;
    }
    Ok(())
}
fn parse_light(ctx: &Context, scene: &mut Scene) -> Result<(),Error> {
    let position = parse_point3(&ctx.get_field("position")?)?;
    let intensity = optional(ctx.get_field("intensity")).map_or_else(
        || Ok(Color::white()),
        |ctx| parse_color(&ctx))?;
    scene.add_light(Light{ position, intensity });
    Ok(())
}

fn parse_roots(
    ctx: &Context,
    scene: &mut Scene,
    objs: BTreeMap<String,ShapeId>,
) -> Result<(),Error> {
    let roots = ctx.as_sequence()?;
    for root in roots {
        let name = root.as_str()?;
        if let Some(sid) = objs.get(&name) {
            scene.add_root(*sid);
        } else {
            return Err(format_err!("object `{}` is not present", name));
        }
    }

    Ok(())
}

// Utility Parsers -------------------------------------------------------------

/// Parse a color as either a hex value, or separate r, g, and b values.
fn parse_color(ctx: &Context) -> Result<Color, Error> {
    if let Ok(hex) = ctx.get_field("hex") {
        let val = hex.as_usize()?;
        let r = (((val >> 16) & 0xff) as f32) / 255.0;
        let g = (((val >> 8) & 0xff) as f32) / 255.0;
        let b = ((val & 0xff) as f32) / 255.0;
        Ok(Color::new(r,g,b))
    } else {
        let r = ctx.get_field("r")?.as_f32()?;
        let g = ctx.get_field("g")?.as_f32()?;
        let b = ctx.get_field("b")?.as_f32()?;
        Ok(Color::new(r,g,b))
    }
}

fn parse_point3(ctx: &Context) -> Result<Point3<f32>,Error> {
    let x = ctx.get_field("x")?.as_f32()?;
    let y = ctx.get_field("y")?.as_f32()?;
    let z = ctx.get_field("z")?.as_f32()?;
    Ok(Point3::new(x,y,z))
}
