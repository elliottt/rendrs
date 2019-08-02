
use yaml_rust::{YamlLoader,Yaml,yaml};

#[macro_use] use failure::{format_err,bail};

use failure::Error;
use nalgebra::{Matrix4,Point3,Vector3};

use std::{
    fs,
    path::Path,
};

use super::Scene;
use crate::camera::Camera;

/// Parse a scene description from a yaml file.
pub fn parser<P>(path: P) -> Result<(Scene,Camera),Error>
    where P: AsRef<Path>,
{
    let bytes = String::from_utf8(fs::read(path)?)?;
    let yaml = YamlLoader::load_from_str(bytes.as_str())?;

    let camera = match yaml.get(1) {
        Some(doc) => parse_camera(doc)?,
        _ => bail!("Missing camera"),
    };

    println!("camera: {:?}", camera);

    let scene = match yaml.get(0) {
        Some(doc) => parse_scene(doc)?,
        _ => bail!("Missing scene"),
    };

    Ok((scene,camera))
}

fn parse_scene(yaml: &Yaml) -> Result<Scene,Error> {
    unimplemented!()
}

fn parse_camera(yaml: &Yaml) -> Result<Camera,Error> {
    let hash = parse_hash(yaml, "camera")?;

    let mut camera = {
        let fov = parse_f32(lookup_hash(hash, "fov")?, "fov")?;
        let width = parse_usize(lookup_hash(hash, "width")?, "width")?;
        let height = parse_usize(lookup_hash(hash, "width")?, "width")?;
        Camera::new(width, height, fov)
    };

    let position = {
        let entry = parse_hash(lookup_hash(hash, "position")?, "position")?;
        let [x,y,z] = parse_xyz(entry)?;
        Point3::new(x,y,z)
    };

    let target = {
        let entry = parse_hash(lookup_hash(hash, "target")?, "target")?;
        let [x,y,z] = parse_xyz(entry)?;
        Point3::new(x,y,z)
    };

    camera.set_transform(Matrix4::look_at_lh(&position, &target, &Vector3::new(0.0, 1.0, 0.0)));

    Ok(camera)
}

fn parse_xyz(hash: &yaml::Hash) -> Result<[f32; 3], Error> {
    let x = parse_f32(lookup_hash(hash, "x")?, "x")?;
    let y = parse_f32(lookup_hash(hash, "y")?, "y")?;
    let z = parse_f32(lookup_hash(hash, "z")?, "z")?;
    Ok([x,y,z])
}

fn lookup_hash<S>(hash: &yaml::Hash, str: S)
    -> Result<&Yaml, Error>
    where S: Into<String>
{
    let key = str.into();
    let err = format_err!("missing entry `{}`", key);
    some_or_err(hash.get(&Yaml::String(key)), err)
}

fn parse_hash<'a>(value: &'a Yaml, entry: &str) -> Result<&'a yaml::Hash, Error> {
    some_or_err(value.as_hash(), format_err!("`{}` must be a hash", entry))
}

fn parse_f32(value: &Yaml, entry: &str) -> Result<f32, Error> {
    let val = some_or_err(value.as_f64(), format_err!("`{}` must be a float", entry))?;
    Ok(val as f32)
}

fn parse_usize(value: &Yaml, entry: &str) -> Result<usize, Error> {
    let val = some_or_err(value.as_i64(), format_err!("`{}` must be an integer", entry))?;
    if val >= 0 {
        Ok(val as usize)
    } else {
        bail!("`{}` must be positive", entry)
    }
}

fn some_or_err<R>(opt: Option<R>, err: Error) -> Result<R, Error> {
    opt.map_or_else(|| Err(err), Ok)
}
