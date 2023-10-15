use crate::error::{Contextable, Result};
use crate::util::{handle_fs_err, handle_fs_res};
use fd_lock::{RwLock, RwLockWriteGuard};
use std::borrow::Cow;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Debug)]
pub struct ProcessInfo<'a> {
    pub run_id: i64,
    pub pid: u32,
    pub script_name: Cow<'a, str>,
    pub args: Cow<'a, [String]>,
    // TODO: env?
}
impl Display for ProcessInfo<'_> {
    fn fmt(&self, w: &mut Formatter<'_>) -> FmtResult {
        write!(w, "{}\n{}\n", self.pid, self.script_name)?;
        let mut first = true;
        for arg in self.args.iter() {
            if !first {
                write!(w, " ")?;
            }
            first = false;
            write!(w, "{}", arg)?;
        }
        Ok(())
    }
}
impl ProcessInfo<'_> {
    fn new(run_id: i64, s: String) -> ProcessInfo<'static> {
        ProcessInfo {
            run_id,
            pid: 0,
            script_name: Cow::Owned("TODO".to_owned()),
            args: Cow::Owned(vec!["TODO".to_owned()]),
        }
    }
    pub fn builder(path: PathBuf) -> Result<ProcessLockCore> {
        let file = handle_fs_res(&[&path], File::open(&path))?;

        Ok(ProcessLockCore {
            lock: RwLock::new(file),
            path,
        })
    }
}

pub struct ProcessLock<'a> {
    core: ProcessLockCore,
    pub process: ProcessInfo<'a>,
}

pub struct ProcessLockCore {
    lock: RwLock<File>,
    pub path: PathBuf,
}

fn try_write<'a>(
    lock: &'a mut RwLock<File>,
    path: &PathBuf,
) -> Result<Option<RwLockWriteGuard<'a, File>>> {
    match lock.try_write() {
        Ok(guard) => Ok(Some(guard)),
        Err(err) => match err.kind() {
            std::io::ErrorKind::WouldBlock => Ok(None),
            _ => Err(handle_fs_err(&[&*path], err)),
        },
    }
}
impl ProcessLockCore {
    pub fn get_can_write(&mut self) -> Result<bool> {
        Ok(try_write(&mut self.lock, &self.path)?.is_some())
    }
    pub fn build(self) -> Result<ProcessInfo<'static>> {
        let mut file = self.lock.into_inner();
        let mut content = String::new();
        handle_fs_res(&[&self.path], file.read_to_string(&mut content)).context("讀取檔案失敗")?;
        let process = ProcessInfo::new(0, content);

        Ok(process)
    }
}

impl<'a> ProcessLock<'a> {
    pub fn new(run_id: i64, script_name: &'a str, args: &'a [String]) -> Result<Self> {
        let path = crate::path::get_process_lock(run_id)?;
        let file = handle_fs_res(&[&path], File::create(&path))?;

        let process = ProcessInfo {
            pid: std::process::id(),
            script_name: script_name.into(),
            args: args.into(),
            run_id,
        };

        Ok(ProcessLock {
            core: ProcessLockCore {
                lock: RwLock::new(file),
                path,
            },
            process,
        })
    }
    pub fn try_write_info(&mut self) -> Result<Option<RwLockWriteGuard<'_, File>>> {
        let mut guard_opt = try_write(&mut self.core.lock, &self.core.path)?;
        if let Some(guard) = guard_opt.as_mut() {
            write!(guard, "{}", self.process)?;
            return Ok(guard_opt);
        }

        log::warn!("{:?} 竟然被其它人鎖住了…？", self.core.path);
        Ok(None)
    }
    pub fn into_process(self) -> ProcessInfo<'a> {
        self.process
    }
}
