// I find ALL solutions on ctate.io not suitable. Let's hand-roll our own...
// * colored: too many heap allocate
// * ansi_term: unmaintained
// * nu_ansi_term: too few people using

use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug, Clone, Copy)]
pub enum Color {
    Red,
    Green,
    Blue,
    BrightBlack,
    Yellow,
}
impl Color {
    pub fn from(s: &str) -> Self {
        Color::Red
    }
}

const BOLD: u8 = 0;
const DIMMED: u8 = 1;
const ITALIC: u8 = 2;
const UNDERLINE: u8 = 3;

#[derive(Default, Debug, Clone, Copy)]
struct Style {
    color: Option<Color>,
    style_map: u8,
}

impl Style {
    fn is_plain(&self) -> bool {
        let Self { color, style_map } = self;
        color.is_none() && *style_map == 0
    }
}

pub struct StyleObj<T> {
    obj: T,
    style: Style,
}

impl<T> StyleObj<T> {
    pub fn bold(mut self) -> Self {
        self.style.style_map |= 1 << BOLD;
        self
    }
    pub fn dimmed(mut self) -> Self {
        self.style.style_map |= 1 << DIMMED;
        self
    }
    pub fn italic(mut self) -> Self {
        self.style.style_map |= 1 << ITALIC;
        self
    }
    pub fn underline(mut self) -> Self {
        self.style.style_map |= 1 << UNDERLINE;
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.style.color = Some(color);
        self
    }
}

pub trait Stylize<T> {
    fn stylize(self) -> StyleObj<T>;
}

impl<T: Display> Stylize<T> for T {
    fn stylize(self) -> StyleObj<T> {
        StyleObj {
            obj: self,
            style: Default::default(),
        }
    }
}

fn fmt_stylemap(f: &mut Formatter<'_>, style_map: u8) -> FmtResult {
    let mut first = true;

    let mut my_write = |s: &'static str| -> FmtResult {
        if !first {
            write!(f, ";")?;
        } else {
            first = false;
        }
        write!(f, "{}", s)
    };

    if style_map & 1 << BOLD != 0 {
        my_write("1")?;
    }
    if style_map & 1 << DIMMED != 0 {
        my_write("2")?;
    }
    if style_map & 1 << ITALIC != 0 {
        my_write("3")?;
    }
    if style_map & 1 << UNDERLINE != 0 {
        my_write("4")?;
    }

    Ok(())
}

impl<T: Display> Display for StyleObj<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.style.is_plain() {
            return write!(f, "{}", self.obj);
        }

        write!(f, "\x1B[")?;
        fmt_stylemap(f, self.style.style_map)?;
        // TODO: color
        write!(f, "m")?;
        write!(f, "{}", self.obj)?;
        write!(f, "\x1B[0m")
    }
}
