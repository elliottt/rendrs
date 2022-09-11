use anyhow::bail;
use nalgebra::{Point3, Unit, Vector3};
use std::iter::Peekable;
use std::path::PathBuf;
use std::str::FromStr;
use std::{collections::HashMap, rc::Rc};

use crate::canvas::Canvas;
use crate::integrator::Whitted;
use crate::scene::MarchConfig;
use crate::{
    camera::{Camera, CanvasInfo, PinholeCamera},
    canvas::Color,
    integrator::Integrator,
    math,
    scene::{MaterialId, NodeId, Scene},
    transform::Transform,
};

use super::lexer::{Lexeme, Lexer, Token};

type Result<T> = std::result::Result<T, anyhow::Error>;

pub fn parse(input: &str) -> Result<(Scene, Vec<Render>)> {
    let mut parser = Parser::new(Lexer::new(input));
    parser.parse()?;
    Ok((parser.scene, parser.renders))
}

/// How to handle the result of rendering.
pub enum Target {
    /// Write the output to this file.
    File { path: PathBuf },

    /// Output the image to the console.
    Ascii,
}

pub struct Render {
    pub target: Target,
    pub canvas: Canvas,
    pub root: NodeId,
    pub integrator: Box<dyn Integrator>,
}

struct Parser<'a> {
    lexer: Peekable<Lexer<'a>>,
    scene: Scene,
    nodes: HashMap<String, NodeId>,
    materials: HashMap<String, MaterialId>,
    cameras: Vec<(String, CanvasInfo, Rc<dyn Camera>)>,
    renders: Vec<Render>,
}

impl<'a> Parser<'a> {
    fn new(lexer: Lexer<'a>) -> Self {
        Self {
            lexer: lexer.peekable(),
            scene: Scene::default(),
            nodes: HashMap::new(),
            materials: HashMap::new(),
            cameras: Vec::new(),
            renders: Vec::new(),
        }
    }

    fn token(&mut self) -> Result<Lexeme> {
        if let Some(lexeme) = self.lexer.next() {
            Ok(lexeme)
        } else {
            bail!("Unexpected EOF")
        }
    }

    fn guard(&mut self, token: Token) -> Result<Lexeme> {
        let tok = self.token()?;
        if tok.token != token {
            bail!("expected a {:?} but found a {:?}", token, tok.token)
        } else {
            Ok(tok)
        }
    }

    fn lparen(&mut self) -> Result<()> {
        self.guard(Token::LParen)?;
        Ok(())
    }

    fn peek_lparen(&mut self) -> bool {
        if let Some(tok) = self.lexer.peek() {
            tok.token == Token::LParen
        } else {
            false
        }
    }

    fn rparen(&mut self) -> Result<()> {
        self.guard(Token::RParen)?;
        Ok(())
    }

    #[inline]
    fn parens<Body, T>(&mut self, body: Body) -> Result<T>
    where
        Body: FnOnce(&mut Self) -> Result<T>,
    {
        self.lparen()?;
        let ret = body(self)?;
        self.rparen()?;
        Ok(ret)
    }

    fn peek_rparen(&mut self) -> bool {
        if let Some(tok) = self.lexer.peek() {
            tok.token == Token::RParen
        } else {
            false
        }
    }

    fn ident(&mut self) -> Result<String> {
        let tok = self.guard(Token::Ident)?;
        Ok(tok.text)
    }

    fn symbol(&mut self) -> Result<String> {
        let tok = self.guard(Token::Symbol)?;
        Ok(tok.text)
    }

    fn string(&mut self) -> Result<String> {
        let tok = self.guard(Token::String)?;
        Ok(String::from(&tok.text[1..tok.text.len() - 1]))
    }

    fn peek_ident(&mut self) -> bool {
        if let Some(tok) = self.lexer.peek() {
            tok.token == Token::Ident
        } else {
            false
        }
    }

    fn number(&mut self) -> Result<f32> {
        if self.peek_lparen() {
            return self.angle();
        }

        let tok = self.guard(Token::Number)?;
        let num = f32::from_str(&tok.text)?;
        Ok(num)
    }

    fn angle(&mut self) -> Result<f32> {
        self.parens(|me| match me.ident()?.as_ref() {
            "degrees" => {
                let deg = me.number()?;
                Ok(math::deg_to_rad(deg))
            }
            "radians" => me.number(),
            angle => bail!("Unknown angle type: {}", angle),
        })
    }

    fn color(&mut self) -> Result<Color> {
        let tok = self.guard(Token::Color)?;
        let text = &tok.text[1..];
        if text.len() != 6 {
            bail!("Invalid hex color: {}", tok.text);
        }

        let val = usize::from_str_radix(&text, 16)?;

        Ok(Color::hex(val))
    }

    fn point(&mut self) -> Result<Point3<f32>> {
        self.parens(|me| {
            let x = me.number()?;
            let y = me.number()?;
            let z = me.number()?;
            Ok(Point3::new(x, y, z))
        })
    }

    fn vector(&mut self) -> Result<Vector3<f32>> {
        self.parens(|me| {
            let x = me.number()?;
            let y = me.number()?;
            let z = me.number()?;
            Ok(Vector3::new(x, y, z))
        })
    }

    fn parse_transforms(&mut self) -> Result<Transform> {
        let mut res = Transform::new();

        while !self.peek_rparen() {
            res = res * &self.parse_transform()?;
        }

        Ok(res)
    }

    fn parse_transform(&mut self) -> Result<Transform> {
        self.parens(|me| match me.ident()?.as_ref() {
            "compose" => me.parse_transforms(),

            "translate" => {
                let x = me.number()?;
                let y = me.number()?;
                let z = me.number()?;
                Ok(Transform::new().translate(&Vector3::new(x, y, z)))
            }

            "rotate" => {
                let axisangle = me.vector()?;
                Ok(Transform::new().rotate(&axisangle))
            }

            "uniform-scale" => {
                let amount = me.number()?;
                Ok(Transform::new().uniform_scale(amount))
            }

            "scale" => {
                let vec = me.vector()?;
                Ok(Transform::new().scale(&vec))
            }

            "look-at" => {
                let eye = me.point()?;
                let target = me.point()?;
                let up = me.vector()?;
                Ok(Transform::look_at(&eye, &target, &up))
            }

            t => bail!("Unknown transform type: {}", t),
        })
    }

    fn parse_pattern(&mut self) -> Result<Color> {
        self.parens(|me| match me.ident()?.as_ref() {
            "color" => me.color(),
            pat => bail!("Unknown pattern type: {}", pat),
        })
    }

    fn parse_material(&mut self) -> Result<MaterialId> {
        if self.peek_ident() {
            let name = self.ident()?;
            if let Some(id) = self.materials.get(&name) {
                return Ok(*id);
            } else {
                bail!("Unknown material: {}", name);
            }
        }

        self.parens(|me| match me.ident()?.as_ref() {
            "phong" => {
                let pattern = me.parse_pattern()?;
                let ambient = me.number()?;
                let diffuse = me.number()?;
                let specular = me.number()?;
                let shininess = me.number()?;
                Ok(me
                    .scene
                    .phong(pattern, ambient, diffuse, specular, shininess))
            }

            name => bail!("Unknown material type: {}", name),
        })
    }

    fn parse_nodes(&mut self) -> Result<Vec<NodeId>> {
        let mut nodes = Vec::new();
        while !self.peek_rparen() {
            nodes.push(self.parse_node()?);
        }

        if nodes.is_empty() {
            bail!("Found an empty node list");
        }

        Ok(nodes)
    }

    fn parse_node(&mut self) -> Result<NodeId> {
        if self.peek_ident() {
            let name = self.ident()?;
            if let Some(id) = self.nodes.get(&name) {
                return Ok(*id);
            } else {
                bail!("Unknown node: {}", name)
            }
        }

        self.parens(|me| match me.ident()?.as_ref() {
            "plane" => {
                let normal = me.vector()?;
                Ok(me.scene.plane(Unit::new_normalize(normal)))
            }

            "sphere" => {
                let radius = me.number()?;
                Ok(me.scene.sphere(radius))
            }

            "box" => {
                let width = me.number()?;
                let height = me.number()?;
                let depth = me.number()?;
                Ok(me.scene.rect(width, height, depth))
            }

            "torus" => {
                let hole = me.number()?;
                let radius = me.number()?;
                Ok(me.scene.torus(hole, radius))
            }

            "group" => {
                let nodes = me.parse_nodes()?;
                Ok(me.scene.group(nodes))
            }

            "union" => {
                let nodes = me.parse_nodes()?;
                Ok(me.scene.union(nodes))
            }

            "subtract" => {
                let left = me.parse_node()?;
                let right = me.parse_node()?;
                Ok(me.scene.subtract(left, right))
            }

            "intersect" => {
                let nodes = me.parse_nodes()?;
                Ok(me.scene.intersect(nodes))
            }

            "smooth-union" => {
                let k = me.number()?;
                let nodes = me.parse_nodes()?;
                Ok(me.scene.smooth_union(k, &nodes))
            }

            "transform" => {
                let t = me.parse_transform()?;
                let sub = me.parse_node()?;
                Ok(me.scene.transform(t, sub))
            }

            "paint" => {
                let mat = me.parse_material()?;
                let node = me.parse_node()?;
                Ok(me.scene.paint(mat, node))
            }

            node => bail!("Unknown node type: {}", node),
        })
    }

    fn parse_light(&mut self) -> Result<()> {
        self.parens(|me| {
            match me.ident()?.as_ref() {
                "diffuse" => {
                    let color = me.color()?;
                    me.scene.diffuse_light(color);
                }

                "point" => {
                    let color = me.color()?;
                    let point = me.point()?;
                    me.scene.point_light(point, color);
                }

                _ => bail!("Failed to parse light"),
            }
            Ok(())
        })
    }

    fn parse_camera(&mut self) -> Result<(CanvasInfo, Rc<dyn Camera>)> {
        if self.peek_ident() {
            let camera_name = self.ident()?;
            let res = self
                .cameras
                .iter()
                .rev()
                .find(|(name, _, _)| *name == camera_name);
            let (info, camera): (CanvasInfo, Rc<dyn Camera>) = if let Some((_, info, camera)) = res
            {
                return Ok((info.clone(), camera.clone()));
            } else {
                bail!("Unknown camera: {}", camera_name);
            };
        }

        self.parens(|me| match me.ident()?.as_ref() {
            "pinhole" => {
                let width = me.number()?;
                let height = me.number()?;
                let t = me.parse_transform()?;
                let fov = me.number()?;
                let info = CanvasInfo::new(width, height);
                let camera = Rc::new(PinholeCamera::new(&info, t, fov)) as Rc<dyn Camera>;
                Ok((info, camera))
            }

            camera => bail!("Unknown camera type: {}", camera),
        })
    }

    fn parse_target(&mut self) -> Result<Target> {
        self.parens(|me| match me.ident()?.as_ref() {
            "file" => {
                let string = me.string()?;
                Ok(Target::File {
                    path: PathBuf::from(string),
                })
            }

            "ascii" => Ok(Target::Ascii),

            target => bail!("Unknown target type: {}", target),
        })
    }

    fn parse_command(&mut self) -> Result<()> {
        self.parens(|me| {
            match me.ident()?.as_ref() {
                "material" => {
                    let name = me.ident()?;
                    let id = me.parse_material()?;
                    me.materials.insert(name, id);
                }

                "node" => {
                    let name = me.ident()?;
                    let id = me.parse_node()?;
                    me.nodes.insert(name, id);
                }

                "light" => {
                    me.parse_light()?;
                }

                "camera" => {
                    let name = me.ident()?;
                    let (info, camera) = me.parse_camera()?;
                    me.cameras.push((name, info, camera));
                }

                "render" => match me.symbol()?.as_ref() {
                    ":whitted" => {
                        let target = me.parse_target()?;

                        let (info, camera) = me.parse_camera()?;
                        let root = me.parse_node()?;

                        let canvas = info.new_canvas();
                        let integrator = Whitted::new(camera, MarchConfig::default(), 10);

                        me.renders.push(Render {
                            target,
                            canvas,
                            root,
                            integrator: Box::new(integrator),
                        })
                    }

                    integrator => bail!("Unknown integrator: {}", integrator),
                },

                command => bail!("Failed to parse command: {}", command),
            }
            Ok(())
        })
    }

    fn parse(&mut self) -> Result<()> {
        while self.lexer.peek().is_some() {
            self.parse_command()?;
        }

        Ok(())
    }
}
