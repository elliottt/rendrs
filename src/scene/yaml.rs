
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

    let scene = parse_scene(&parsed)?;
    let cameras = parse_cameras(&parsed)?;

    Ok((scene,cameras))
}

fn parse_scene(parsed: &Value) -> Result<Scene,Error> {
    let mut scene = Scene::new();

    let pats = parsed.get("patterns").map_or_else(
        || Ok(BTreeMap::new()),
        |val| parse_pats(&mut scene, val))?;

    let mats = parsed.get("materials").map_or_else(
        || Ok(BTreeMap::new()),
        |val| parse_mats(&mut scene, val))?;

    let objs = parsed.get("objects").map_or_else(
        || Ok(BTreeMap::new()),
        |val| parse_objs(&mut scene, &pats, &mats, val))?;

    parsed.get("lights").map_or_else(
        || Err(format_err!("No lights defined in the scene")),
        |val| parse_lights(&mut scene, val))?;

    parsed.get("scene").map_or_else(
        || Err(format_err!("No objects placed in the scene")),
        |val| parse_roots(&mut scene, objs, val))?;

    Ok(scene)
}

fn parse_cameras(parsed: &Value) -> Result<Vec<Camera>,Error> {
    let entries = parsed.get("cameras").map_or_else(
        || Err(format_err!("`cameras` missing from scene")),
        |val| val.as_sequence().map_or_else(
            || Err(format_err!("`cameras` must be a sequence")),
            |vals| Ok(vals)))?;

    let mut cameras = Vec::with_capacity(entries.len());
    for entry in entries {
        let camera = parse_camera(entry)?;
        cameras.push(camera);
    }

    Ok(cameras)
}

fn parse_camera(parsed: &Value) -> Result<Camera,Error> {
    if let Some(val) = parsed.get("perspective") {
        parse_perspective(val)
    } else {
        Err(format_err!("unknown camera type"))
    }
}

fn parse_perspective(val: &Value) -> Result<Camera,Error> {
    let width = val.get("width").map_or_else(
        || Err(format_err!("`width` field missing from `perspective`")),
        |val| val.as_u64().map_or_else(
            || Err(format_err!("`width` must be an integer")),
            |val| Ok(val as usize)))?;

    let height = val.get("height").map_or_else(
        || Err(format_err!("`height` field missing from `perspective`")),
        |val| val.as_u64().map_or_else(
            || Err(format_err!("`height` must be an integer")),
            |val| Ok(val as usize)))?;

    let fov = val.get("fov").map_or_else(
        || Err(format_err!("`fov` field missing from `perspective`")),
        |val| val.as_f64().map_or_else(
            || Err(format_err!("`fov` must be a float")),
            |val| Ok(val as f32)))?;

    let position = val.get("position").map_or_else(
        || Err(format_err!("`position` missing from `perspective`")),
        |val| parse_point3(val))?;

    let target = val.get("target").map_or_else(
        || Err(format_err!("`position` missing from `perspective`")),
        |val| parse_point3(val))?;

    let mut camera = Camera::new(width, height, fov);

    camera.set_transform(Matrix4::look_at_lh(&position, &target, &Vector3::new(0.0, 1.0, 0.0)));

    Ok(camera)
}

#[derive(Debug)]
enum ParsedPat {
    Solid(Color),
    Striped(String,String),
}

fn parse_pats(scene: &mut Scene, val: &Value)
    -> Result<BTreeMap<String,PatternId>, Error>
{

    let entries = val.as_sequence().map_or_else(
        || Err(format_err!("`patterns` must be a sequence of pattern entries")),
        |seq| Ok(seq))?;

    let mut pat_map = BTreeMap::new();

    // build the work queue
    let mut work = Vec::with_capacity(entries.len());
    for entry in entries {
        let name = entry.get("name").and_then(|val| val.as_str()).map_or_else(
            || Err(format_err!("`name` is required in each pattern")),
            |name| Ok(name))?;

        let parsed = parse_pat(entry)?;
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

fn parse_pat(val: &Value) -> Result<ParsedPat,Error> {

    if let Some(val) = val.get("solid") {
        let color = parse_color(val)?;
        return Ok(ParsedPat::Solid(color));
    } else if let Some(val) = val.get("striped") {
        let a = val.get(0).and_then(|val| val.as_str()).map_or_else(
            || Err(format_err!("failed to parse name in `striped`")),
            |name| Ok(name.to_string()))?;
        let b = val.get(1).and_then(|val| val.as_str()).map_or_else(
            || Err(format_err!("failed to parse name in `striped`")),
            |name| Ok(name.to_string()))?;
        return Ok(ParsedPat::Striped(a,b));
    }

    Err(format_err!("Unknown pattern type"))
}

fn parse_mats(scene: &mut Scene, val: &Value) -> Result<BTreeMap<String,MaterialId>,Error> {
    let entries = val.as_sequence().map_or_else(
        || Err(format_err!("`materials` must be a sequence of material entries")),
        |seq| Ok(seq))?;

    let mut mat_map = BTreeMap::new();

    for entry in entries {
        let name = entry.get("name").and_then(|val| val.as_str()).map_or_else(
            || Err(format_err!("missing `name` from material entry")),
            |val| Ok(val.to_string()))?;

        let mid = parse_mat(scene, entry)?;

        mat_map.insert(name, mid);
    }

    Ok(mat_map)
}

fn parse_mat(scene: &mut Scene, val: &Value) -> Result<MaterialId,Error> {
    let ambient = val.get("ambient").and_then(|val| val.as_f64()).map_or_else(
        || Err(format_err!("`ambient` missing from material")),
        |val| Ok(val as f32))?;
    let diffuse = val.get("diffuse").and_then(|val| val.as_f64()).map_or_else(
        || Err(format_err!("`diffuse` missing from material")),
        |val| Ok(val as f32))?;
    let specular = val.get("specular").and_then(|val| val.as_f64()).map_or_else(
        || Err(format_err!("`specular` missing from material")),
        |val| Ok(val as f32))?;
    let shininess = val.get("shininess").and_then(|val| val.as_f64()).map_or_else(
        || Err(format_err!("`shininess` missing from material")),
        |val| Ok(val as f32))?;
    Ok(scene.add_material(Material{ ambient, diffuse, specular, shininess }))
}

fn parse_objs(
    scene: &mut Scene,
    pats: &BTreeMap<String,PatternId>,
    mats: &BTreeMap<String,MaterialId>,
    val: &Value
) -> Result<BTreeMap<String,ShapeId>,Error> {
    let entries = val.as_sequence().map_or_else(
        || Err(format_err!("`materials` must be a sequence of material entries")),
        |seq| Ok(seq))?;

    let mut obj_map = BTreeMap::new();

    // build up work queue
    let mut work = Vec::with_capacity(entries.len());
    for entry in entries {
        let name = entry.get("name").and_then(|val| val.as_str()).map_or_else(
            || Err(format_err!("missing name for `object` definition")),
            |val| Ok(val.to_string()))?;

        let obj = parse_obj(entry)?;

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

fn parse_obj(val: &Value) -> Result<ParsedObj,Error> {
    if let Some(_) = val.get("sphere") {
        Ok(ParsedObj::Sphere)
    } else if let Some(_) = val.get("plane") {
        Ok(ParsedObj::Plane)
    } else if let Some(args) = val.get("material") {
        let pattern = args.get("pattern").and_then(|val| val.as_str()).map(|val| val.to_string());
        let material = args.get("material").and_then(|val| val.as_str()).map(|val| val.to_string());
        let name = args.get("object").map_or_else(
            || Err(format_err!("`material` requires a `object`")),
            |val| val.as_str().map_or_else(
                || Err(format_err!("`object` must be a string")),
                |val| Ok(val.to_string())))?;
        Ok(ParsedObj::Material(pattern,material,name))
    } else {
        Err(format_err!("Unknown shape type"))
    }
}

fn parse_lights(scene: &mut Scene, val: &Value) -> Result<(),Error> {
    val.as_sequence().map_or_else(
        || Err(format_err!("`lights` must be a list of light descriptions")),
        |lights| {
            for light in lights {
                parse_light(scene, light)?;
            }
            Ok(())
        })
}
fn parse_light(scene: &mut Scene, val: &Value) -> Result<(),Error> {
    let position = val.get("position").map_or_else(
        || Err(format_err!("`light` requires a `position`")),
        parse_point3)?;

    let intensity = val.get("intensity").map_or_else(
        || Ok(Color::white()),
        parse_color)?;

    scene.add_light(Light{ position, intensity });

    Ok(())
}

fn parse_roots(
    scene: &mut Scene,
    objs: BTreeMap<String,ShapeId>,
    val: &Value,
) -> Result<(),Error> {
    let roots = val.as_sequence().map_or_else(
        || Err(format_err!("`scene` must be a list of object names")),
        |val| Ok(val))?;

    for root in roots {
        let name = root.as_str().map_or_else(
            || Err(format_err!("elements of `scene` must be strings")),
            |val| Ok(val.to_string()))?;

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
fn parse_color(val: &Value) -> Result<Color, Error> {
    if let Some(val) = val.get("hex") {
        let hex = val.as_u64().map_or_else(
            || Err(format_err!("`hex` must be a hex number")),
            |val| Ok(val))?;

        let r = (((hex >> 16) & 0xff) as f32) / 255.0;
        let g = (((hex >> 8) & 0xff) as f32) / 255.0;
        let b = ((hex & 0xff) as f32) / 255.0;

        Ok(Color::new(r,g,b))
    } else {
        let r = val.get("r").and_then(|val| val.as_f64()).map_or_else(
            || Err(format_err!("`r` missing from color spec")),
            |val| Ok(val as f32))?;
        let g = val.get("g").and_then(|val| val.as_f64()).map_or_else(
            || Err(format_err!("`g` missing from color spec")),
            |val| Ok(val as f32))?;
        let b = val.get("b").and_then(|val| val.as_f64()).map_or_else(
            || Err(format_err!("`b` missing from color spec")),
            |val| Ok(val as f32))?;
        Ok(Color::new(r,g,b))
    }
}

fn parse_point3(val: &Value) -> Result<Point3<f32>,Error> {
    let mut pos = Point3::origin();

    val.get("x").map_or_else(
        || Err(format_err!("`x` missing")),
        |val| val.as_f64().map_or_else(
            || Err(format_err!("`x` must be a float")),
            |val| {
                pos.x = val as f32;
                Ok(())
            })
        )?;

    val.get("y").map_or_else(
        || Err(format_err!("`x` missing")),
        |val| val.as_f64().map_or_else(
            || Err(format_err!("`x` must be a float")),
            |val| {
                pos.y = val as f32;
                Ok(())
            })
        )?;

    val.get("z").map_or_else(
        || Err(format_err!("`z` missing")),
        |val| val.as_f64().map_or_else(
            || Err(format_err!("`z` must be a float")),
            |val| {
                pos.z = val as f32;
                Ok(())
            })
        )?;

    Ok(pos)
}
