// I find ALL solutions on ctate.io not suitable. Let's hand-roll our own...
// * colored: too many heap allocate
// * ansi_term: unmaintained
// * nu_ansi_term: too few people using

use std::fmt::{Display, Error as FmtError, Formatter, Result as FmtResult};

#[derive(Debug, Clone, Copy)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}
impl Color {
    pub fn from(s: &str) -> Self {
        match s {
            "black" => Color::Black,
            "red" => Color::Red,
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "blue" => Color::Blue,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            "white" => Color::White,
            "bright black" => Color::BrightBlack,
            "bright red" => Color::BrightRed,
            "bright green" => Color::BrightGreen,
            "bright yellow" => Color::BrightYellow,
            "bright blue" => Color::BrightBlue,
            "bright magenta" => Color::BrightMagenta,
            "bright cyan" => Color::BrightCyan,
            "bright white" => Color::BrightWhite,

            _ => {
                let ret = Color::White;
                log::warn!("錯誤的顏色 {}，改用 {:?}", s, ret);
                ret
            }
        }
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
    pub fn done(&self) -> () {
        ()
    }
    pub fn bold(&mut self) -> &mut Self {
        self.style.style_map |= 1 << BOLD;
        self
    }
    pub fn dimmed(&mut self) -> &mut Self {
        self.style.style_map |= 1 << DIMMED;
        self
    }
    pub fn italic(&mut self) -> &mut Self {
        self.style.style_map |= 1 << ITALIC;
        self
    }
    pub fn underline(&mut self) -> &mut Self {
        self.style.style_map |= 1 << UNDERLINE;
        self
    }
    pub fn color(&mut self, color: Color) -> &mut Self {
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

fn fmt_stylemap(f: &mut Formatter<'_>, style_map: u8) -> Result<bool, FmtError> {
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

    Ok(first)
}
fn fmt_color(f: &mut Formatter, color: Color) -> FmtResult {
    let s = match color {
        Color::Black => "30",
        Color::Red => "31",
        Color::Green => "32",
        Color::Yellow => "33",
        Color::Blue => "34",
        Color::Magenta => "35",
        Color::Cyan => "36",
        Color::White => "37",
        Color::BrightBlack => "90",
        Color::BrightRed => "91",
        Color::BrightGreen => "92",
        Color::BrightYellow => "93",
        Color::BrightBlue => "94",
        Color::BrightMagenta => "95",
        Color::BrightCyan => "96",
        Color::BrightWhite => "97",
    };
    write!(f, "{}", s)
}

impl<T: Display> Display for StyleObj<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.style.is_plain() {
            return write!(f, "{}", self.obj);
        }

        write!(f, "\x1B[")?;
        let first = fmt_stylemap(f, self.style.style_map)?;

        if let Some(color) = self.style.color {
            if !first {
                write!(f, ";")?;
            }
            fmt_color(f, color)?;
        }

        write!(f, "m")?;
        write!(f, "{}", self.obj)?;
        write!(f, "\x1B[0m")
    }
}
