
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
    obj::Obj,
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

    fn is_seq(&self) -> bool {
        self.focus.is_sequence()
    }

    fn sequence_len(&self) -> Result<usize,Error> {
        self.focus.as_sequence().map_or_else(
            || Err(format_err!("expected a sequence")),
            |val| Ok(val.len()))
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
    if let Ok(ctx) = ctx.get_field("perspective") {
        parse_perspective(&ctx)
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

    let num_samples = optional(ctx.get_field("samples")).map_or_else(
        || Ok(1), |ctx| ctx.as_usize())?;

    let mut camera = Camera::new(width, height, fov, num_samples.max(1));
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
        transform: Matrix4<f32>,
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

                ParsedPat::Transform{ ref transform, ref pattern } => {
                    if let Some(cid) = pat_map.get(pattern) {
                        let pid = scene.add_pattern(Pattern::transform(transform, *cid));
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
        let (transform,_) = parse_transform(&ctx)?;
        let pattern = parse_pat_subtree(&ctx.get_field("pattern")?, work)?;
        work.push(name, ParsedPat::Transform{ transform, pattern });
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

                ParsedObj::Transform{ ref transform, scale_factor, ref object } => {
                    if let Some(oid) = obj_map.get(object) {
                        let sid = scene.add(Shape::transform(transform, scale_factor, *oid));
                        obj_map.insert(name,sid);
                        continue;
                    }
                },

                ParsedObj::Group{ ref objects } => {
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
                        let sid = scene.add(Shape::group(scene, resolved));
                        obj_map.insert(name,sid);
                        continue;
                    }
                },

                ParsedObj::Union{ ref smooth, ref objects } => {
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
                        let sid =
                            if let Some(k) = smooth {
                                build_smooth_union(scene, *k, &resolved)
                            } else {
                                Ok(scene.add(Shape::union(scene, resolved)))
                            }?;
                        obj_map.insert(name,sid);
                        continue;
                    }
                },

                ParsedObj::Subtract{ ref smooth, ref first, ref second } => {
                    if let Some(aid) = obj_map.get(first) {
                        if let Some(bid) = obj_map.get(second) {
                            let sid =
                                if let Some(k) = smooth {
                                    scene.add(Shape::smooth_subtract(*k, *aid, *bid))
                                } else {
                                    scene.add(Shape::subtract(*aid, *bid))
                                };
                            obj_map.insert(name,sid);
                            continue;
                        }
                    }
                },

                ParsedObj::Intersect{ ref objects } => {
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
                        let sid = scene.add(Shape::intersect(scene, resolved));
                        obj_map.insert(name,sid);
                        continue;
                    }
                }

                ParsedObj::Onion{ thickness, ref object } => {
                    if let Some(oid) = obj_map.get(object) {
                        let sid = scene.add(Shape::onion(thickness, *oid));
                        obj_map.insert(name, sid);
                        continue;
                    }
                },

                ParsedObj::Model{ ref file } => {
                    use std::{io::BufReader,fs::File};
                    let file = File::open(file)?;
                    let model = Obj::parse(BufReader::new(file))?;
                    let sid = model.add_to_scene(scene)?;
                    obj_map.insert(name, sid);
                    continue;
                },

                ParsedObj::Rounded{ rad, ref object } => {
                    if let Some(oid) = obj_map.get(object) {
                        let sid = scene.add(Shape::rounded(rad, *oid));
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

fn build_smooth_union(scene: &mut Scene, k: f32, resolved: &[ShapeId]) -> Result<ShapeId,Error> {
    let pivot = resolved.len() / 2;

    let left = &resolved[0..pivot];
    let right = &resolved[pivot..];

    let first =
        match left.len() {
            0 => Err(format_err!("union with smoothing requires at least two nodes")),
            1 => Ok(left[0]),
            _ => build_smooth_union(scene, k, left),
        }?;

    let second =
        match right.len() {
            0 => Err(format_err!("union with smoothing requires at least two nodes")),
            1 => Ok(right[0]),
            _ => build_smooth_union(scene, k, right),
        }?;

    Ok(scene.add(Shape::smooth_union(k, first, second)))
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
        transform: Matrix4<f32>,
        scale_factor: f32,
        object: ParsedName
    },
    Group{
        objects: Vec<ParsedName>,
    },
    Union{
        smooth: Option<f32>,
        objects: Vec<ParsedName>,
    },
    Subtract{
        smooth: Option<f32>,
        first: ParsedName,
        second: ParsedName,
    },
    Intersect{
        objects: Vec<ParsedName>,
    },
    Onion{
        thickness: f32,
        object: ParsedName,
    },
    Model{
        file: String,
    },
    Rounded{
        rad: f32,
        object: ParsedName,
    },
}

fn parse_obj(
    ctx: &Context,
    work: &mut ParseQueue<ParsedObj>,
    name: ParsedName,
) -> Result<(),Error> {
    if let Ok(_ctx) = ctx.get_field("sphere") {
        work.push(name,ParsedObj::PrimShape{ prim: PrimShape::Sphere });
    } else if let Ok(ctx) = ctx.get_field("torus") {
        let radius = optional(ctx.get_field("radius")).map_or_else(
            || Ok(0.5), |ctx| ctx.as_f32())?;
        let hole = optional(ctx.get_field("hole")).map_or_else(
            || Ok(0.5), |ctx| ctx.as_f32())?;
        work.push(name,ParsedObj::PrimShape{ prim: PrimShape::Torus{ radius, hole }});
    } else if let Ok(ctx) = ctx.get_field("cube") {
        let size = optional(ctx.get_field("size")).map_or_else(
            || Ok(1.0), |ctx| ctx.as_f32())?;
        work.push(name,ParsedObj::PrimShape{ prim: PrimShape::RectangularPrism{
            width: size,
            height: size,
            depth: size,
        }});
    } else if let Ok(_ctx) = ctx.get_field("plane") {
        work.push(name,ParsedObj::PrimShape{ prim: PrimShape::XZPlane });
    } else if let Ok(ctx) = ctx.get_field("cylinder") {
        let radius = optional(ctx.get_field("radius")).map_or_else(
            || Ok(1.0), |ctx| ctx.as_f32())?;
        let length = optional(ctx.get_field("length")).map_or_else(
            || Ok(1.0), |ctx| ctx.as_f32())?;
        work.push(name, ParsedObj::PrimShape{ prim: PrimShape::Cylinder{ radius, length }});
    } else if let Ok(ctx) = ctx.get_field("triangle") {
        let len = ctx.sequence_len()?;
        if len != 3 {
            return Err(format_err!("`triangle` requires three points"));
        }
        let a = parse_point3(&ctx.get_at(0)?)?;
        let b = parse_point3(&ctx.get_at(1)?)?;
        let c = parse_point3(&ctx.get_at(2)?)?;
        work.push(name,ParsedObj::PrimShape{ prim: PrimShape::triangle(&a,&b,&c) });
    } else if let Ok(args) = ctx.get_field("material") {
        let pattern = optional(args.get_field("pattern")).map_or_else(
            || Ok(None), |ctx| ctx.as_str().map(|name| Some(ParsedName::String(name))))?;
        let material = optional(args.get_field("material")).map_or_else(
            || Ok(None), |ctx| ctx.as_str().map(Some))?;
        let object = parse_subtree(&args.get_field("object")?, work)?;
        work.push(name,ParsedObj::Material{ pattern, material, object });
    } else if let Ok(ctx) = ctx.get_field("model") {
        let file = ctx.as_str()?;
        work.push(name,ParsedObj::Model{ file });
    } else if let Ok(args) = ctx.get_field("transform") {
        let (transform,scale_factor) = parse_transform(&args)?;
        let object = parse_subtree(&args.get_field("object")?, work)?;
        work.push(name, ParsedObj::Transform{ transform, scale_factor, object });
    } else if let Ok(ctx) = ctx.get_field("group") {
        let mut objects = Vec::new();
        let entries = ctx.as_sequence()?;
        for entry in entries {
            objects.push(parse_subtree(&entry, work)?);
        }
        work.push(name, ParsedObj::Group{ objects });
    } else if let Ok(args) = ctx.get_field("union") {
        let smooth = optional(args.get_field("smooth")).map_or_else(
            || Ok(None), |ctx| ctx.as_f32().map(Some))?;
        let mut objects = Vec::new();
        let entries = args.get_field("objects")?.as_sequence()?;
        for entry in entries {
            objects.push(parse_subtree(&entry, work)?);
        }
        work.push(name, ParsedObj::Union{ smooth, objects });
    } else if let Ok(args) = ctx.get_field("intersect") {
        let mut objects = Vec::new();
        let entries = args.get_field("objects")?.as_sequence()?;
        for entry in entries {
            objects.push(parse_subtree(&entry, work)?);
        }
        work.push(name, ParsedObj::Intersect{ objects });
    } else if let Ok(args) = ctx.get_field("subtract") {
        let smooth = optional(args.get_field("smooth")).map_or_else(
            || Ok(None), |ctx| ctx.as_f32().map(Some))?;
        let entries = args.get_field("objects")?;
        let first = parse_subtree(&entries.get_at(0)?, work)?;
        let second = parse_subtree(&entries.get_at(1)?, work)?;
        work.push(name, ParsedObj::Subtract{ smooth, first, second });
    } else if let Ok(args) = ctx.get_field("onion") {
        let thickness = args.get_field("thickness")?.as_f32()?;
        let object = parse_subtree(&args.get_field("object")?, work)?;
        work.push(name, ParsedObj::Onion{ thickness, object });
    } else if let Ok(ctx) = ctx.get_field("rounded") {
        let rad = ctx.get_field("rad")?.as_f32()?;
        let object = parse_subtree(&ctx.get_field("object")?, work)?;
        work.push(name, ParsedObj::Rounded{ rad, object });
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

    let mut nodes = Vec::new();

    for root in ctx.as_sequence()? {
        let name = parse_obj_name(&root)?;
        if let Some(sid) = objs.get(&name) {
            nodes.push(*sid);
        } else {
            // TODO: don't use the debug formatter here
            return Err(format_err!("object `{:?}` is not present", name));
        }
    }

    let gid = scene.add(Shape::group(scene, nodes));
    scene.set_root(gid);

    Ok(())
}

// Utility Parsers -------------------------------------------------------------

/// Parse a transformation matrix.
fn parse_transform(ctx: &Context) -> Result<(Matrix4<f32>,f32),Error> {
    let scale = optional(ctx.get_field("scale")).map_or_else(
        || Ok(None), |ctx| parse_scale(&ctx).map(Some))?;
    let rotation = optional(ctx.get_field("rotation")).map_or_else(
        || Ok(None), |ctx| parse_rotation(&ctx).map(Some))?;
    let translation = optional(ctx.get_field("translation")).map_or_else(
        || Ok(None), |ctx| parse_vector3(&ctx).map(Some))?;

    let mut trans = Matrix4::identity();
    let mut scale_factor = 1.0;

    if let Some(vec) = scale {
        scale_factor = vec.x.min(vec.y).min(vec.z);
        trans.append_nonuniform_scaling_mut(&vec);
    }

    if let Some((vec,angle)) = rotation {
        let axis = Unit::new_normalize(vec);
        trans *= Matrix4::from_axis_angle(&axis, angle);
    }

    if let Some(vec) = translation {
        trans.append_translation_mut(&vec);
    }

    Ok((trans,scale_factor))
}

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

fn parse_three(ctx: &Context, def: f32)-> Result<(f32,f32,f32),Error> {
    if ctx.is_seq() {
        let x = ctx.get_at(0)?.as_f32()?;
        let y = ctx.get_at(1)?.as_f32()?;
        let z = ctx.get_at(2)?.as_f32()?;
        Ok((x,y,z))
    } else {
        let x = optional(ctx.get_field("x")).map_or_else(
            || Ok(def), |ctx| ctx.as_f32())?;
        let y = optional(ctx.get_field("y")).map_or_else(
            || Ok(def), |ctx| ctx.as_f32())?;
        let z = optional(ctx.get_field("z")).map_or_else(
            || Ok(def), |ctx| ctx.as_f32())?;
        Ok((x,y,z))
    }
}

fn parse_point3(ctx: &Context) -> Result<Point3<f32>,Error> {
    let (x,y,z) = parse_three(ctx, 0.0)?;
    Ok(Point3::new(x,y,z))
}

fn parse_vector3(ctx: &Context) -> Result<Vector3<f32>,Error> {
    let (x,y,z) = parse_three(ctx, 0.0)?;
    Ok(Vector3::new(x,y,z))
}

fn parse_scale(ctx: &Context) -> Result<Vector3<f32>,Error> {
    if let Ok(ctx) = ctx.get_field("uniform") {
        let uniform = ctx.as_f32()?;
        Ok(Vector3::new(uniform, uniform, uniform))
    } else {
        let x = optional(ctx.get_field("x")).map_or_else(
            || Ok(1.0), |ctx| ctx.as_f32())?;
        let y = optional(ctx.get_field("y")).map_or_else(
            || Ok(1.0), |ctx| ctx.as_f32())?;
        let z = optional(ctx.get_field("z")).map_or_else(
            || Ok(1.0), |ctx| ctx.as_f32())?;
        Ok(Vector3::new(x,y,z))
    }
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
