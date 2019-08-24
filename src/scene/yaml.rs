
use failure::{Error,format_err};
use serde_yaml::Value;
use nalgebra::{Unit,Point3,Vector3,Matrix4};

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

    fn is_hash(&self) -> bool {
        self.focus.is_mapping()
    }
}

/// Parsed names, plus fresh names for nested structures.
#[derive(Eq,PartialEq,Ord,PartialOrd,Debug,Clone)]
enum ParsedName{
    String(String),
    Fresh(usize),
}

/// A queue of parsed objects with names to resolve.
struct ParseQueue<T> {
    next: usize,
    work: Vec<(ParsedName,T)>,
}

impl<T> ParseQueue<T> {
    fn new() -> Self {
        ParseQueue { next: 0, work: Vec::new() }
    }

    fn fresh_name(&mut self) -> ParsedName {
        let name = ParsedName::Fresh(self.next);
        self.next += 1;
        name
    }

    fn push(&mut self, name: ParsedName, val: T) {
        self.work.push((name,val))
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
    let fov = parse_angle(&ctx.get_field("fov")?)?;
    let position = parse_point3(&ctx.get_field("position")?)?;
    let target = parse_point3(&ctx.get_field("target")?)?;

    let mut camera = Camera::new(width, height, fov);
    camera.set_transform(Matrix4::look_at_lh(&position, &target, &Vector3::new(0.0, 1.0, 0.0)));

    Ok(camera)
}

#[derive(Debug)]
enum ParsedPat {
    Solid{
        color: Color
    },
    Striped{
        first: ParsedName,
        second: ParsedName,
    },
    Gradient{
        first: ParsedName,
        second: ParsedName,
    },
    Circles{
        first: ParsedName,
        second: ParsedName,
    },
    Checkers{
        first: ParsedName,
        second: ParsedName,
    },
    Transform{
        translation: Option<Vector3<f32>>,
        rotation: Option<(Vector3<f32>,f32)>,
        pattern: ParsedName,
    }
}

fn parse_pats(ctx: &Context, scene: &mut Scene)
    -> Result<BTreeMap<ParsedName,PatternId>, Error>
{

    let entries = ctx.as_sequence()?;

    let mut pat_map = BTreeMap::new();

    // build the work queue
    let mut pq = ParseQueue::new();
    for entry in entries {
        let name = entry.get_field("name")?.as_str()?;
        parse_pat(&entry, &mut pq, ParsedName::String(name))?;
    }

    let mut work = pq.work;
    let mut next = Vec::with_capacity(work.len());
    while !work.is_empty() {
        let start_len = work.len();

        while let Some((name,parsed)) = work.pop() {
            match parsed {
                ParsedPat::Solid{ ref color } => {
                    let pid = scene.add_pattern(Pattern::Solid{ color: color.clone() });
                    pat_map.insert(name, pid);
                    continue;
                },

                ParsedPat::Striped{ ref first, ref second } => {
                    if let Some(a) = pat_map.get(first) {
                        if let Some(b) = pat_map.get(second) {
                            let pid = scene.add_pattern(Pattern::stripe(*a, *b));
                            pat_map.insert(name, pid);
                            continue;
                        }
                    }
                },

                ParsedPat::Gradient{ ref first, ref second } => {
                    if let Some(a) = pat_map.get(first) {
                        if let Some(b) = pat_map.get(second) {
                            let pid = scene.add_pattern(Pattern::gradient(*a, *b));
                            pat_map.insert(name, pid);
                            continue;
                        }
                    }
                },

                ParsedPat::Circles{ ref first, ref second } => {
                    if let Some(a) = pat_map.get(first) {
                        if let Some(b) = pat_map.get(second) {
                            let pid = scene.add_pattern(Pattern::circles(*a, *b));
                            pat_map.insert(name, pid);
                            continue;
                        }
                    }
                },

                ParsedPat::Checkers{ ref first, ref second } => {
                    if let Some(a) = pat_map.get(first) {
                        if let Some(b) = pat_map.get(second) {
                            let pid = scene.add_pattern(Pattern::checkers(*a, *b));
                            pat_map.insert(name, pid);
                            continue;
                        }
                    }
                },

                ParsedPat::Transform{ ref translation, ref rotation, ref pattern } => {
                    if let Some(cid) = pat_map.get(pattern) {
                        let mut trans =
                            if let Some((vec,angle)) = rotation {
                                let axis = Unit::new_normalize(vec.clone());
                                Matrix4::from_axis_angle(&axis, *angle)
                            } else {
                                Matrix4::identity()
                            };

                        if let Some(vec) = translation {
                            trans = trans.append_translation(vec);
                        }

                        let pid = scene.add_pattern(Pattern::transform(trans, *cid));
                        pat_map.insert(name, pid);
                        continue;
                    }
                },
            }

            next.push((name,parsed));
        }

        if start_len == next.len() {
            return Err(format_err!("invalid patterns: naming cycle, or missing named pattern"));
        }

        std::mem::swap(&mut next, &mut work);
    }

    Ok(pat_map)
}

fn parse_pat(
    ctx: &Context,
    work: &mut ParseQueue<ParsedPat>,
    name: ParsedName,
) -> Result<(),Error> {
    if let Ok(ctx) = ctx.get_field("solid") {
        let color = parse_color(&ctx)?;
        work.push(name, ParsedPat::Solid{ color });
    } else if let Ok(ctx) = ctx.get_field("gradient") {
        let first = parse_pat_subtree(&ctx.get_at(0)?, work)?;
        let second = parse_pat_subtree(&ctx.get_at(1)?, work)?;
        work.push(name, ParsedPat::Gradient{ first, second });
    } else if let Ok(ctx) = ctx.get_field("striped") {
        let first = parse_pat_subtree(&ctx.get_at(0)?, work)?;
        let second = parse_pat_subtree(&ctx.get_at(1)?, work)?;
        work.push(name, ParsedPat::Striped{ first, second });
    } else if let Ok(ctx) = ctx.get_field("circles") {
        let first = parse_pat_subtree(&ctx.get_at(0)?, work)?;
        let second = parse_pat_subtree(&ctx.get_at(1)?, work)?;
        work.push(name, ParsedPat::Circles{ first, second });
    } else if let Ok(ctx) = ctx.get_field("checkers") {
        let first = parse_pat_subtree(&ctx.get_at(0)?, work)?;
        let second = parse_pat_subtree(&ctx.get_at(1)?, work)?;
        work.push(name, ParsedPat::Checkers{ first, second });
    } else if let Ok(ctx) = ctx.get_field("transform") {
        let translation = optional(ctx.get_field("translation")).map_or_else(
            || Ok(None), |ctx| parse_vector3(&ctx).map(Some))?;
        let rotation = optional(ctx.get_field("rotation")).map_or_else(
            || Ok(None), |ctx| parse_rotation(&ctx).map(Some))?;
        let pattern = parse_pat_subtree(&ctx.get_field("pattern")?, work)?;
        work.push(name, ParsedPat::Transform{ translation, rotation, pattern });
    } else {
        return Err(format_err!("Unknown pattern type"))
    }
    Ok(())
}

fn parse_pat_subtree(
    ctx: &Context,
    work: &mut ParseQueue<ParsedPat>,
) -> Result<ParsedName,Error> {
    if let Ok(name) = ctx.as_str() {
        Ok(ParsedName::String(name.to_string()))
    } else if ctx.is_hash() {
        let name = work.fresh_name();
        parse_pat(ctx, work, name.clone())?;
        Ok(name.clone())
    } else {
        Err(format_err!("unable to parse pattern"))
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
    let def = Material::default();

    let ambient = optional(ctx.get_field("ambient")).map_or_else(
        || Ok(def.ambient), |ctx| ctx.as_f32())?;
    let diffuse = optional(ctx.get_field("diffuse")).map_or_else(
        || Ok(def.diffuse), |ctx| ctx.as_f32())?;
    let specular = optional(ctx.get_field("specular")).map_or_else(
        || Ok(def.specular), |ctx| ctx.as_f32())?;
    let shininess = optional(ctx.get_field("shininess")).map_or_else(
        || Ok(def.shininess), |ctx| ctx.as_f32())?;
    let reflective = optional(ctx.get_field("reflective")).map_or_else(
        || Ok(def.reflective), |ctx| ctx.as_f32())?;
    let transparent = optional(ctx.get_field("transparent")).map_or_else(
        || Ok(def.transparent), |ctx| ctx.as_f32())?;
    let refractive_index = optional(ctx.get_field("refractive-index")).map_or_else(
        || Ok(def.refractive_index), |ctx| ctx.as_f32())?;
    Ok(scene.add_material(Material::new(
                ambient,
                diffuse,
                specular,
                shininess,
                reflective,
                transparent,
                refractive_index,
            )))
}

fn parse_objs(
    ctx: &Context,
    scene: &mut Scene,
    pats: &BTreeMap<ParsedName,PatternId>,
    mats: &BTreeMap<String,MaterialId>,
) -> Result<BTreeMap<ParsedName,ShapeId>,Error> {
    let entries = ctx.as_sequence()?;

    // build up work queue
    let mut pq = ParseQueue::new();
    for entry in entries {
        let name = parse_obj_name(&entry.get_field("name")?)?;
        parse_obj(&entry, &mut pq, name)?;
    }

    let mut obj_map = BTreeMap::new();
    let mut work = pq.work;
    let mut next = Vec::with_capacity(work.len());
    while !work.is_empty() {
        let start_len = work.len();

        while let Some((name,parsed)) = work.pop() {
            match parsed {
                ParsedObj::PrimShape{ prim } => {
                    let sid = scene.add(Shape::PrimShape{ shape: prim });
                    obj_map.insert(name,sid);
                    continue;
                },

                ParsedObj::Material{ ref pattern, ref material, ref object } => {
                    if let Some(oid) = obj_map.get(object) {
                        let pid = match pattern {
                            None => Ok(scene.default_pattern),
                            Some(ref name) =>
                                pats.get(name).map_or_else(
                                    || Err(format_err!("missing pattern `{:?}`", name)),
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
                        continue;
                    }
                },

                ParsedObj::Transform{ ref translation, ref rotation, ref object } => {
                    if let Some(oid) = obj_map.get(object) {
                        let mut trans =
                            if let Some((vec,angle)) = rotation {
                                let axis = Unit::new_normalize(vec.clone());
                                Matrix4::from_axis_angle(&axis, *angle)
                            } else {
                                Matrix4::identity()
                            };

                        if let Some(vec) = translation {
                            trans = trans.append_translation(vec);
                        }

                        let sid = scene.add(Shape::transform(&trans, *oid));

                        obj_map.insert(name,sid);
                        continue;
                    }
                },

                ParsedObj::Union{ ref objects } => {
                    let mut resolved = Vec::with_capacity(objects.len());
                    let mut all_resolved = true;
                    for obj in objects {
                        if let Some(oid) = obj_map.get(obj) {
                            resolved.push(*oid);
                        } else {
                            all_resolved = false;
                            break;
                        }
                    }

                    if all_resolved {
                        let sid = scene.add(Shape::union(resolved));
                        obj_map.insert(name,sid);
                        continue;
                    }
                },

                ParsedObj::Subtract{ ref first, ref second } => {
                    if let Some(aid) = obj_map.get(first) {
                        if let Some(bid) = obj_map.get(second) {
                            let sid = scene.add(Shape::subtract(*aid, *bid));
                            obj_map.insert(name,sid);
                            continue;
                        }
                    }
                },

                ParsedObj::UniformScale{ ref amount, ref object } => {
                    if let Some(oid) = obj_map.get(object) {
                        let sid = scene.add(Shape::uniform_scaling(*amount, *oid));
                        obj_map.insert(name, sid);
                        continue;
                    }
                },
            }

            next.push((name,parsed));
        }

        if start_len == next.len() {
            return Err(format_err!("invalid objects: naming cycle, or missing named object"));
        }

        std::mem::swap(&mut next, &mut work);
    }

    Ok(obj_map)
}

enum ParsedObj {
    PrimShape{
        prim: PrimShape,
    },
    Material{ 
        pattern: Option<ParsedName>,
        material: Option<String>,
        object: ParsedName,
    },
    Transform{
        translation: Option<Vector3<f32>>,
        rotation: Option<(Vector3<f32>,f32)>,
        object: ParsedName
    },
    Union{
        objects: Vec<ParsedName>,
    },
    Subtract{
        first: ParsedName,
        second: ParsedName,
    },
    UniformScale{
        amount: f32,
        object: ParsedName,
    },
}

fn parse_obj(
    ctx: &Context,
    work: &mut ParseQueue<ParsedObj>,
    name: ParsedName,
) -> Result<(),Error> {
    if let Ok(args) = ctx.get_field("prim") {
        let sort = args.as_str()?;
        match sort.as_str() {
            "sphere" =>
                work.push(name,ParsedObj::PrimShape{ prim: PrimShape::Sphere }),
            "cylinder" =>
                work.push(name,ParsedObj::PrimShape{ prim: PrimShape::Cylinder }),
            "plane" =>
                work.push(name,ParsedObj::PrimShape{ prim: PrimShape::XZPlane }),
            "cube" =>
                work.push(name,ParsedObj::PrimShape{ prim: PrimShape::Cube }),
            other =>
                return Err(format_err!("unknown primitive `{}`", other)),
        };
    } else if let Ok(args) = ctx.get_field("material") {
        let pattern = optional(args.get_field("pattern")).map_or_else(
            || Ok(None), |ctx| ctx.as_str().map(|name| Some(ParsedName::String(name))))?;
        let material = optional(args.get_field("material")).map_or_else(
            || Ok(None), |ctx| ctx.as_str().map(Some))?;
        let object = parse_subtree(&args.get_field("object")?, work)?;
        work.push(name,ParsedObj::Material{ pattern, material, object });
    } else if let Ok(args) = ctx.get_field("transform") {
        let translation = optional(args.get_field("translation")).map_or_else(
            || Ok(None), |ctx| parse_vector3(&ctx).map(Some))?;
        let rotation = optional(args.get_field("rotation")).map_or_else(
            || Ok(None), |ctx| parse_rotation(&ctx).map(Some))?;
        let object = parse_subtree(&args.get_field("object")?, work)?;
        work.push(name, ParsedObj::Transform{ translation, rotation, object });
    } else if let Ok(args) = ctx.get_field("union") {
        let mut objects = Vec::new();
        let entries = args.as_sequence()?;
        for entry in entries {
            objects.push(parse_subtree(&entry, work)?);
        }
        work.push(name, ParsedObj::Union{ objects });
    } else if let Ok(args) = ctx.get_field("subtract") {
        let first = parse_subtree(&args.get_at(0)?, work)?;
        let second = parse_subtree(&args.get_at(1)?, work)?;
        work.push(name, ParsedObj::Subtract{ first, second });
    } else if let Ok(args) = ctx.get_field("uniform-scale") {
        let amount = args.get_field("amount")?.as_f32()?;
        let object = parse_subtree(&args.get_field("object")?, work)?;
        work.push(name, ParsedObj::UniformScale{ amount, object });
    } else {
        return Err(format_err!("Unknown shape type"));
    }

    Ok(())
}

fn parse_subtree(
    ctx: &Context,
    work: &mut ParseQueue<ParsedObj>
) -> Result<ParsedName,Error> {
    if let Ok(obj_ref) = parse_obj_name(ctx) {
        Ok(obj_ref)
    } else if ctx.is_hash() {
        let fresh = work.fresh_name();
        parse_obj(ctx, work, fresh.clone())?;
        Ok(fresh)
    } else {
        Err(format_err!("Unable to parse `object`"))
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
    objs: BTreeMap<ParsedName,ShapeId>,
) -> Result<(),Error> {
    let roots = ctx.as_sequence()?;
    for root in roots {
        let name = parse_obj_name(&root)?;
        if let Some(sid) = objs.get(&name) {
            scene.add_root(*sid);
        } else {
            // TODO: don't use the debug formatter here
            return Err(format_err!("object `{:?}` is not present", name));
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
    let x = optional(ctx.get_field("x")).map_or_else(
        || Ok(0.0), |ctx| ctx.as_f32())?;
    let y = optional(ctx.get_field("y")).map_or_else(
        || Ok(0.0), |ctx| ctx.as_f32())?;
    let z = optional(ctx.get_field("z")).map_or_else(
        || Ok(0.0), |ctx| ctx.as_f32())?;
    Ok(Point3::new(x,y,z))
}

fn parse_vector3(ctx: &Context) -> Result<Vector3<f32>,Error> {
    let x = optional(ctx.get_field("x")).map_or_else(
        || Ok(0.0), |ctx| ctx.as_f32())?;
    let y = optional(ctx.get_field("y")).map_or_else(
        || Ok(0.0), |ctx| ctx.as_f32())?;
    let z = optional(ctx.get_field("z")).map_or_else(
        || Ok(0.0), |ctx| ctx.as_f32())?;
    Ok(Vector3::new(x,y,z))
}

fn parse_rotation(ctx: &Context) -> Result<(Vector3<f32>,f32),Error> {
    let vec = parse_vector3(ctx)?;
    let angle = parse_angle(ctx)?;
    Ok((vec,angle))
}

/// Parses either `radians` or `degrees` out of the context, with a preference for `radians`.
fn parse_angle(ctx: &Context) -> Result<f32,Error> {
    if let Ok(args) = ctx.get_field("radians") {
        args.as_f32()
    } else if let Ok(args) = ctx.get_field("degrees") {
        args.as_f32().map(|val| std::f32::consts::PI * val / 180.0)
    } else {
        Err(format_err!("missing `radians` or `degrees`"))
    }
}

fn parse_obj_name(ctx: &Context) -> Result<ParsedName,Error> {
    let name = ctx.as_str()?;
    Ok(ParsedName::String(name))
}
