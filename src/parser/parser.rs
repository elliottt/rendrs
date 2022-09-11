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

pub struct Render {
    pub canvas: Canvas,
    pub root: NodeId,
    pub integrator: Box<dyn Integrator>,
    pub path: PathBuf,
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

    fn rparen(&mut self) -> Result<()> {
        self.guard(Token::RParen)?;
        Ok(())
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
        Ok(String::from(&tok.text[1..tok.text.len()-1]))
    }

    fn peek_ident(&mut self) -> bool {
        if let Some(tok) = self.lexer.peek() {
            tok.token == Token::Ident
        } else {
            false
        }
    }

    fn number(&mut self) -> Result<f32> {
        let tok = self.guard(Token::Number)?;
        let num = f32::from_str(&tok.text)?;
        Ok(num)
    }

    fn angle(&mut self) -> Result<f32> {
        self.lparen()?;

        let angle = match self.ident()?.as_ref() {
            "degrees" => {
                let deg = self.number()?;
                math::deg_to_rad(deg)
            }
            "radians" => self.number()?,
            angle => bail!("Unknown angle type: {}", angle),
        };

        self.rparen()?;
        Ok(angle)
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
        self.lparen()?;
        let x = self.number()?;
        let y = self.number()?;
        let z = self.number()?;
        self.rparen()?;
        Ok(Point3::new(x, y, z))
    }

    fn vector(&mut self) -> Result<Vector3<f32>> {
        self.lparen()?;
        let x = self.number()?;
        let y = self.number()?;
        let z = self.number()?;
        self.rparen()?;
        Ok(Vector3::new(x, y, z))
    }

    fn parse_transform(&mut self) -> Result<Transform> {
        self.lparen()?;

        let t = match self.ident()?.as_ref() {
            "translate" => {
                let x = self.number()?;
                let y = self.number()?;
                let z = self.number()?;
                Transform::new().translate(&Vector3::new(x, y, z))
            }

            "look-at" => {
                let eye = self.point()?;
                let target = self.point()?;
                let up = self.vector()?;
                Transform::look_at(&eye, &target, &up)
            }

            t => bail!("Unknown transform type: {}", t),
        };

        self.rparen()?;
        Ok(t)
    }

    fn parse_pattern(&mut self) -> Result<Color> {
        self.lparen()?;

        let pat = match self.ident()?.as_ref() {
            "color" => self.color(),
            pat => bail!("Unknown pattern type: {}", pat),
        };

        self.rparen()?;

        pat
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

        self.lparen()?;

        let node = match self.ident()?.as_ref() {
            "phong" => {
                let pattern = self.parse_pattern()?;
                let ambient = self.number()?;
                let diffuse = self.number()?;
                let specular = self.number()?;
                let shininess = self.number()?;
                self.scene
                    .phong(pattern, ambient, diffuse, specular, shininess)
            }

            name => bail!("Unknown material type: {}", name),
        };

        self.rparen()?;

        Ok(node)
    }

    fn parse_nodes(&mut self) -> Result<Vec<NodeId>> {
        let mut nodes = Vec::new();
        while !self.peek_rparen() {
            nodes.push(self.parse_node()?);
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

        self.lparen()?;

        let node = match self.ident()?.as_ref() {
            "plane" => {
                let normal = self.vector()?;
                self.scene.plane(Unit::new_normalize(normal))
            }

            "sphere" => {
                let radius = self.number()?;
                self.scene.sphere(radius)
            }

            "box" => {
                let width = self.number()?;
                let height = self.number()?;
                let depth = self.number()?;
                self.scene.rect(width, height, depth)
            }

            "torus" => {
                let hole = self.number()?;
                let radius = self.number()?;
                self.scene.torus(hole, radius)
            }

            "group" => {
                let nodes = self.parse_nodes()?;
                self.scene.group(nodes)
            }

            "union" => {
                let nodes = self.parse_nodes()?;
                self.scene.union(nodes)
            }

            "smooth-union" => {
                let k = self.number()?;
                let left = self.parse_node()?;
                let right = self.parse_node()?;
                self.scene.smooth_union(k, left, right)
            }

            "transform" => {
                let t = self.parse_transform()?;
                let sub = self.parse_node()?;
                self.scene.transform(t, sub)
            }

            "paint" => {
                let mat = self.parse_material()?;
                let node = self.parse_node()?;
                self.scene.paint(mat, node)
            }

            node => bail!("Unknown node type: {}", node),
        };

        self.rparen()?;
        Ok(node)
    }

    fn parse_light(&mut self) -> Result<()> {
        self.lparen()?;

        match self.ident()?.as_ref() {
            "diffuse" => {
                let color = self.color()?;
                self.scene.diffuse_light(color);
            }

            "point" => {
                let color = self.color()?;
                let point = self.point()?;
                self.scene.point_light(point, color);
            }

            _ => bail!("Failed to parse light"),
        }

        self.rparen()
    }

    fn parse_camera(&mut self) -> Result<(CanvasInfo, Rc<dyn Camera>)> {
        self.lparen()?;

        let res = match self.ident()?.as_ref() {
            "pinhole" => {
                let width = self.number()?;
                let height = self.number()?;
                let t = self.parse_transform()?;
                let fov = self.angle()?;
                let info = CanvasInfo::new(width, height);
                let camera = Rc::new(PinholeCamera::new(&info, t, fov)) as Rc<dyn Camera>;
                (info, camera)
            }

            camera => bail!("Unknown camera type: {}", camera),
        };

        self.rparen()?;

        Ok(res)
    }

    fn parse_command(&mut self) -> Result<()> {
        self.lparen()?;

        match self.ident()?.as_ref() {
            "material" => {
                let name = self.ident()?;
                let id = self.parse_material()?;
                self.materials.insert(name, id);
            }

            "node" => {
                let name = self.ident()?;
                let id = self.parse_node()?;
                self.nodes.insert(name, id);
            }

            "light" => {
                self.parse_light()?;
            }

            "camera" => {
                let name = self.ident()?;
                let (info, camera) = self.parse_camera()?;
                self.cameras.push((name, info, camera));
            }

            "render" => match self.symbol()?.as_ref() {
                ":whitted" => {
                    // TODO: maybe allow specifying a camera here?
                    let camera_name = self.ident()?;
                    let res = self
                        .cameras
                        .iter()
                        .rev()
                        .find(|(name, _, _)| *name == camera_name);
                    let (info, camera): (CanvasInfo, Rc<dyn Camera>) =
                        if let Some((_, info, camera)) = res {
                            (info.clone(), camera.clone())
                        } else {
                            bail!("Unknown camera: {}", camera_name);
                        };

                    let root = self.parse_node()?;

                    let canvas = info.new_canvas();
                    let integrator = Whitted::new(camera, MarchConfig::default(), 10);

                    let path = PathBuf::from(self.string()?);

                    self.renders.push(Render {
                        canvas,
                        root,
                        integrator: Box::new(integrator),
                        path,
                    })
                }

                integrator => bail!("Unknown integrator: {}", integrator),
            },

            command => bail!("Failed to parse command: {}", command),
        }

        self.rparen()?;

        Ok(())
    }

    fn parse(&mut self) -> Result<()> {
        while self.lexer.peek().is_some() {
            self.parse_command()?;
        }

        Ok(())
    }
}
