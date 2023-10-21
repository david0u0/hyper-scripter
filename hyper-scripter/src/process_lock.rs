use crate::error::{Contextable, Error, Result};
use crate::util::{handle_fs_err, handle_fs_res};
use fd_lock::{RwLock, RwLockWriteGuard};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Debug)]
struct ProcessInfoWrite<'a> {
    pid: u32,
    script_id: i64,
    script_name: &'a str,
    args: &'a [String], // TODO: env?
}

#[derive(Debug)]
pub struct ProcessInfoRead {
    raw_file_content: String,
    file_content_start: usize,

    // 以下成員皆包含於 `file_content()` 中
    pub pid: u32,
    pub script_id: i64,
}
impl ProcessInfoRead {
    fn new(raw_file_content: String) -> Result<ProcessInfoRead> {
        log::debug!("處理進程資訊： {}", raw_file_content);
        let space1 = raw_file_content
            .find(' ')
            .ok_or_else(|| Error::msg("can't find 1st space"))?;
        let space2 = raw_file_content[space1 + 1..]
            .find(' ')
            .ok_or_else(|| Error::msg("can't find 2nd space"))?
            + space1
            + 1;

        let pid = raw_file_content[..space1].parse()?;
        let script_id = raw_file_content[space1 + 1..space2].parse()?;

        Ok(ProcessInfoRead {
            script_id,
            pid,
            raw_file_content,
            file_content_start: space2 + 1,
        })
    }
    pub fn file_content(&self) -> &'_ str {
        &self.raw_file_content[self.file_content_start..]
    }
}

pub struct ProcessLockWrite<'a> {
    core: ProcessLockCore,
    process: ProcessInfoWrite<'a>,
}

pub struct ProcessLockCore {
    run_id: i64,
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
    pub fn build(mut self) -> Result<ProcessLockRead> {
        let mut file = self.lock.into_inner();
        let mut content = String::new();
        handle_fs_res(&[&self.path], file.read_to_string(&mut content)).context("讀取檔案失敗")?;
        self.lock = RwLock::new(file);

        let process = ProcessInfoRead::new(content)?;
        Ok(ProcessLockRead {
            core: self,
            process,
        })
    }
}

impl<'a> ProcessLockWrite<'a> {
    pub fn new(
        run_id: i64,
        script_id: i64,
        script_name: &'a str,
        args: &'a [String],
    ) -> Result<Self> {
        let path = crate::path::get_process_lock(run_id)?;
        let file = handle_fs_res(&[&path], File::create(&path))?;

        let process = ProcessInfoWrite {
            pid: std::process::id(),
            script_id,
            script_name,
            args,
        };

        Ok(ProcessLockWrite {
            core: ProcessLockCore {
                lock: RwLock::new(file),
                run_id,
                path,
            },
            process,
        })
    }
    pub fn try_write_info(&mut self) -> Result<Option<RwLockWriteGuard<'_, File>>> {
        let mut guard_opt = try_write(&mut self.core.lock, &self.core.path)?;
        if let Some(guard) = guard_opt.as_mut() {
            write!(
                guard,
                "{} {} {}",
                self.process.pid, self.process.script_id, self.process.script_name
            )?;
            for arg in self.process.args.iter() {
                write!(guard, " {}", arg)?;
            }
            return Ok(guard_opt);
        }

        log::warn!("{:?} 竟然被其它人鎖住了…？", self.core.path);
        Ok(None)
    }
}

pub struct ProcessLockRead {
    core: ProcessLockCore,
    pub process: ProcessInfoRead,
}
impl ProcessLockRead {
    pub fn get_run_id(&self) -> i64 {
        self.core.run_id
    }
    pub fn builder(path: PathBuf, file_name: &str) -> Result<ProcessLockCore> {
        let file = handle_fs_res(&[&path], File::open(&path))?;
        let run_id = file_name.parse()?;

        Ok(ProcessLockCore {
            lock: RwLock::new(file),
            run_id,
            path,
        })
    }
    pub fn wait_write(&mut self) -> Result {
        let _g = self.core.lock.write()?;
        Ok(())
    }
}
