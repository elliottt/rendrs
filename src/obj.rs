use anyhow::{bail, Error};
use nalgebra::{Point3, Unit, Vector3};

type Result<T> = std::result::Result<T, Error>;

#[derive(Default, Debug)]
pub struct Face {
    pub vertices: Vec<Point3<f32>>,
}

#[derive(Debug)]
pub struct Group {
    pub name: String,
    pub faces: Vec<Face>,
}

impl Group {
    pub fn new(name: String) -> Self {
        Group {
            name,
            faces: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Obj {
    pub groups: Vec<Group>,
}

impl Obj {
    pub fn parse<B: AsRef<str>>(buf: B) -> Result<Obj> {
        let mut parser = Parser::new(buf.as_ref());

        let mut groups = Vec::new();

        groups.push(Group::new(String::new()));
        let mut group = groups.last_mut().unwrap();

        while let Ok(cmd) = parser.command() {
            match cmd {
                Command::Group { name } if group.faces.is_empty() => group.name = name,
                Command::Group { name } => {
                    groups.push(Group::new(name));
                    group = groups.last_mut().unwrap();
                }
                Command::Face { face } => group.faces.push(face),
            }
        }

        Ok(Obj { groups })
    }
}

struct Parser<'a> {
    buf: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    offset: usize,
    vertices: Vec<Point3<f32>>,
}

impl<'a> Parser<'a> {
    fn new(buf: &'a str) -> Self {
        Self {
            buf,
            chars: buf.char_indices().peekable(),
            offset: 0,
            vertices: Vec::new(),
        }
    }

    fn pos(&self) -> usize {
        self.offset
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    fn consume(&mut self) -> Option<char> {
        self.chars.next().map(|(off, c)| {
            self.offset += 1;
            c
        })
    }

    fn consume_if<P: FnOnce(char) -> bool>(&mut self, pred: P) -> Option<char> {
        self.chars.next_if(|(_, c)| pred(*c)).map(|(ix, c)| {
            self.offset += 1;
            c
        })
    }

    fn consume_while<P: FnMut(bool, char) -> bool>(&mut self, mut pred: P) -> (usize, usize) {
        let start = self.pos();

        while let Some((ix, _)) = self.chars.next_if(|(ix, c)| pred(*ix > start, *c)) {
            self.offset += 1;
        }

        (start, self.pos())
    }

    fn skip_line(&mut self) {
        while self.consume_if(|c| c != '\n').is_some() {}
    }

    /// Skips space in the input. Returns true/false to indicate if there is more input to consume
    /// on this line.
    fn skip_space(&mut self) -> bool {
        while let Some(c) = self.peek_char() {
            if c == '#' {
                self.skip_line();
                self.consume();
                return false;
            }

            if c == '\n' {
                println!("found eol");
                self.consume();
                return false;
            }

            if !c.is_whitespace() {
                return true;
            }

            self.consume();
        }

        // breaking out of the loop means that we ran out of input
        false
    }

    fn token(&mut self) -> Result<&str> {
        self.skip_space();

        let (start, end) = self.consume_while(|_, c| !c.is_whitespace());
        let len = end - start;

        if len == 0 {
            bail!("Failed to parse a token");
        } else {
            Ok(&self.buf[start..end])
        }
    }

    fn f32(&mut self) -> Result<f32> {
        let tok = self.token()?;
        let num = tok.parse()?;
        Ok(num)
    }

    fn vertex(&mut self) -> Result<Point3<f32>> {
        let tok = self.token()?;
        let idx = tok.parse::<usize>()?;
        Ok(self.vertices[idx - 1])
    }

    fn command(&mut self) -> Result<Command> {
        loop {
            match self.token()? {
                "g" => {
                    let name = self.token()?;
                    return Ok(Command::Group {
                        name: String::from(name),
                    });
                }

                "v" => {
                    let point = Point3::new(self.f32()?, self.f32()?, self.f32()?);
                    self.vertices.push(point);
                }

                "vn" | "vt" => self.skip_line(),

                "f" => {
                    let mut face = Face::default();
                    while self.skip_space() {
                        println!("vertex!");
                        face.vertices.push(self.vertex()?);
                    }
                    println!("done!");
                    return Ok(Command::Face { face });
                }

                tok => bail!("Unknown command: {}", tok),
            }
        }
    }
}

enum Command {
    Group { name: String },
    Face { face: Face },
}

#[test]
fn test_parse_token() {
    let text = "g hello\n";
    let mut p = Parser::new(&text);
    assert_eq!("g", p.token().unwrap());
    assert_eq!("hello", p.token().unwrap());
}

#[test]
fn test_parse_group() {
    let text = "g hello\n";
    let mut p = Parser::new(&text);
    assert!(matches!(p.command().unwrap(), Command::Group { name } if name == "hello" ));
}

#[test]
fn test_parse_vertex() {
    let text = "v 1 1 1";
    let mut p = Parser::new(&text);
    let _ = p.command();
    assert_eq!(1, p.vertices.len());
    assert_eq!(Point3::new(1., 1., 1.), p.vertices[0]);
}

#[test]
fn test_parse_face() {
    let text = "v 1 1 1\nv 2 2 2\nv 3 3 3\nf 3 1 2 # comment";
    let mut p = Parser::new(&text);

    let cmd = p.command();
    assert_eq!(3, p.vertices.len());
    assert_eq!(Point3::new(1., 1., 1.), p.vertices[0]);
    assert_eq!(Point3::new(2., 2., 2.), p.vertices[1]);
    assert_eq!(Point3::new(3., 3., 3.), p.vertices[2]);

    let cmd = cmd.unwrap();
    match cmd {
        Command::Face {
            face: Face { vertices },
        } => {
            println!("{:?}", vertices);
            assert_eq!(Point3::new(3., 3., 3.), vertices[0]);
            assert_eq!(Point3::new(1., 1., 1.), vertices[1]);
            assert_eq!(Point3::new(2., 2., 2.), vertices[2]);
        }

        _ => panic!("Failed to parse a face"),
    }
}
