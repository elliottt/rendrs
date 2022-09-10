use anyhow::bail;
use nalgebra::{Point3, Vector3};
use std::collections::HashMap;
use std::iter::Peekable;
use std::str::FromStr;

use super::{
    lexer::{Lexeme, Lexer, Token},
    Error,
};
use crate::{
    scene::{NodeId, Scene},
    transform::Transform,
};

type Result<T> = std::result::Result<T, anyhow::Error>;

pub fn parse(input: &str) -> Result<Scene> {
    let mut parser = Parser::new(Lexer::new(input));
    parser.parse()?;
    Ok(parser.scene)
}

struct Parser<'a> {
    lexer: Peekable<Lexer<'a>>,
    scene: Scene,
    nodes: HashMap<String, NodeId>,
}

impl<'a> Parser<'a> {
    fn new(lexer: Lexer<'a>) -> Self {
        Self {
            lexer: lexer.peekable(),
            scene: Scene::default(),
            nodes: HashMap::new(),
        }
    }

    fn token(&mut self) -> Result<Lexeme> {
        if let Some(lexeme) = self.lexer.next() {
            Ok(lexeme)
        } else {
            bail!(Error::ParserError)
        }
    }

    fn guard(&mut self, token: Token) -> Result<Lexeme> {
        let tok = self.token()?;
        if tok.token != token {
            bail!("expected a {:?}", token)
        } else {
            Ok(tok)
        }
    }

    fn lparen(&mut self) -> Result<()> {
        self.guard(Token::LParen);
        Ok(())
    }

    fn rparen(&mut self) -> Result<()> {
        self.guard(Token::RParen)?;
        Ok(())
    }

    fn ident(&mut self) -> Result<String> {
        let tok = self.guard(Token::Ident)?;
        Ok(tok.text)
    }

    fn number(&mut self) -> Result<f32> {
        let tok = self.guard(Token::Number)?;
        let num = f32::from_str(&tok.text)?;
        Ok(num)
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

    fn parse_prim(&mut self) -> Result<NodeId> {
        self.lparen()?;

        let node = match self.ident()?.as_ref() {
            "sphere" => {
                let radius = self.number()?;
                self.scene.sphere(radius)
            }
            _ => bail!(Error::ParserError),
        };

        self.rparen()?;

        Ok(node)
    }

    fn peek_ident(&mut self) -> bool {
        if let Some(tok) = self.lexer.peek() {
            tok.token == Token::Ident
        } else {
            false
        }
    }

    fn peek_rparen(&mut self) -> bool {
        if let Some(tok) = self.lexer.peek() {
            tok.token == Token::RParen
        } else {
            false
        }
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
            "prim" => self.parse_prim()?,

            "group" => {
                let nodes = self.parse_nodes()?;
                self.scene.group(nodes)
            }

            "union" => {
                let nodes = self.parse_nodes()?;
                self.scene.union(nodes)
            }

            "translate" => {
                let vec = self.vector()?;
                let sub = self.parse_node()?;
                self.scene.transform(Transform::new().translate(&vec), sub)
            }

            _ => bail!(Error::ParserError),
        };

        self.rparen()?;
        Ok(node)
    }

    fn parse_command(&mut self) -> Result<()> {
        self.lparen()?;

        match self.ident()?.as_ref() {
            "node" => {
                let name = self.ident()?;
                let id = self.parse_node()?;
                self.nodes.insert(name, id);
            }

            _ => bail!(Error::ParserError),
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
