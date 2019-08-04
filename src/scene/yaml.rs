
use failure::{Error,format_err};
use serde_yaml::Value;

use std::{
    collections::BTreeMap,
    fs,
    path::Path,
};

use crate::{
    canvas::Color,
    camera::Camera,
    material::{Material,MaterialId},
    pattern::{Pattern,PatternId},
    scene::Scene,
    shapes::{ShapeId,Shape,PrimShape},
};


pub fn parse<P>(path: P)
    -> Result<(Scene,Camera), Error>
    where P: AsRef<Path>
{
    let bytes = String::from_utf8(fs::read(path)?)?;
    let parsed: Value = serde_yaml::from_str(bytes.as_str())?;

    let mut scene = Scene::new();

    let pats = parsed.get("patterns").map_or_else(
        || Ok(BTreeMap::new()),
        |val| parse_pats(&mut scene, val))?;

    let mats = parsed.get("materials").map_or_else(
        || Ok(BTreeMap::new()),
        |val| parse_mats(&mut scene, val))?;

    let objs = parsed.get("objects").map_or_else(
        || Ok(BTreeMap::new()),
        |val| parse_objs(&mut scene, val))?;

    println!("patterns: {:?}", pats);
    println!("materials: {:?}", mats);
    println!("objs: {:?}", objs);

    unimplemented!()
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

fn parse_color(val: &Value) -> Result<Color, Error> {
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

fn parse_objs(scene: &mut Scene, val: &Value) -> Result<BTreeMap<String,ShapeId>,Error> {
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
}

fn parse_obj(val: &Value) -> Result<ParsedObj,Error> {
    if let Some(_) = val.get("sphere") {
        Ok(ParsedObj::Sphere)
    } else {
        Err(format_err!("Unknown shape type"))
    }
}
