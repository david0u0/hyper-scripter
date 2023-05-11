pub struct FmtWrite<W: std::fmt::Write>(pub W);
pub struct IoWrite<W: std::io::Write>(pub W);
pub trait Writable {
    type Fmt: std::fmt::Write;
    type IO: std::io::Write;
    fn as_fmt(&mut self) -> Option<&mut Self::Fmt>;
    fn as_io(&mut self) -> Option<&mut Self::IO>;
}

impl<W: std::fmt::Write> Writable for FmtWrite<W> {
    type Fmt = W;
    type IO = std::io::Stdout;
    fn as_fmt(&mut self) -> Option<&mut Self::Fmt> {
        Some(&mut self.0)
    }
    fn as_io(&mut self) -> Option<&mut Self::IO> {
        None
    }
}

impl<W: std::io::Write> Writable for IoWrite<W> {
    type Fmt = String;
    type IO = W;
    fn as_fmt(&mut self) -> Option<&mut Self::Fmt> {
        None
    }
    fn as_io(&mut self) -> Option<&mut Self::IO> {
        Some(&mut self.0)
    }
}

macro_rules! write_writable {
    ($dst:expr, $($arg:tt)*) => {{
        use $crate::error::Error;
        use std::io::Write as IOWrite;
        use std::fmt::Write as FmtWrite;
        if let Some(w) = $dst.as_fmt() {
            write!(w, $($arg)*).map_err(|e| Into::<Error>::into(e))
        } else if let Some(w) = $dst.as_io() {
            write!(w, $($arg)*).map_err(|e| Into::<Error>::into(e))
        } else {
            panic!()
        }
    }};
}
pub(crate) use write_writable;

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_writable() {
        let mut s = String::new();
        let mut w = FmtWrite(&mut s);
        write_writable!(w, "{}", 123).unwrap();
        assert_eq!("123", s);

        let mut stdout = std::io::stdout().lock();
        let mut w = IoWrite(&mut stdout);
        write_writable!(w, "{}", 123).unwrap();
    }
}
