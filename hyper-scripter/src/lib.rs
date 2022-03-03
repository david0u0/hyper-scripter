#![feature(command_access)]

#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate derive_more;
pub mod args;
pub mod config;
pub mod db;
pub mod error;
mod error_display;
pub mod extract_msg;
pub mod fuzzy;
pub mod list;
pub mod migration;
pub mod path;
pub mod query;
pub mod script;
pub mod script_repo;
pub mod script_time;
pub mod script_type;
pub mod state;
pub mod tag;
pub mod util;

pub const SEP: &str = "/";
pub enum Either<T, U> {
    One(T),
    Two(U),
}

#[derive(Copy, Clone)]
pub struct MyRaw<T>(T);
unsafe impl<T> Send for MyRaw<T> {}
impl MyRaw<*const str> {
    pub unsafe fn as_str(&self) -> &str {
        &*self.0
    }
}
impl<U: ?Sized> MyRaw<*const U> {
    fn new(r: &U) -> MyRaw<*const U> {
        MyRaw(r as *const _)
    }
}
impl<T: Copy> MyRaw<T> {
    pub fn get(&self) -> T {
        self.0
    }
}
