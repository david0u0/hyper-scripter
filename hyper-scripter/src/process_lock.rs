use crate::error::{Contextable, Error, Result};
use crate::util::{handle_fs_err, handle_fs_res};
use fd_lock::{RwLock, RwLockWriteGuard};
use std::fs::File;
use std::io::Seek;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

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
        log::debug!("處理進程資訊：{:?}", raw_file_content);

        let new_line = raw_file_content
            .find('\n')
            .ok_or_else(|| Error::msg("can't find new line"))?;
        let (pid, script_id) = raw_file_content[..new_line]
            .split_once(' ')
            .ok_or_else(|| Error::msg("can't find space"))?;

        let pid = pid.parse()?;
        let script_id = script_id.parse()?;

        Ok(ProcessInfoRead {
            script_id,
            pid,
            raw_file_content,
            file_content_start: new_line + 1,
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
                "{} {}\n{}",
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
    pub fn mark_sucess(guard: Option<RwLockWriteGuard<'_, File>>) {
        if let Some(guard) = guard {
            if let Err(err) = guard.set_len(0) {
                log::warn!("Failed to mark file lock as success: {}", err)
            }
        }
    }
    pub fn get_path(&self) -> &Path {
        &self.core.path
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
    pub fn wait_write(mut self) -> Result {
        let g = self.core.lock.write()?;
        drop(g);

        let mut file = self.core.lock.into_inner();
        file.rewind()?;
        if file.bytes().next().is_none() {
            Ok(())
        } else {
            log::warn!("檔案鎖內容沒被砍掉代表進程沒有正常結束…");
            Err(Error::ScriptError(1))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const SCRIPT_NAME: &str = "this-name";
    #[test]
    fn test_process_lock() {
        const RUN_ID: i64 = 1;
        const SCRIPT_ID: i64 = 2;
        let file_path = crate::path::get_process_lock(RUN_ID).unwrap();

        let mut write_lock = ProcessLockWrite::new(RUN_ID, SCRIPT_ID, SCRIPT_NAME, &[]).unwrap();
        let mut read_core =
            ProcessLockRead::builder(file_path.clone(), &RUN_ID.to_string()).unwrap();

        assert!(read_core.get_can_write().unwrap());

        let write_guard = write_lock.try_write_info().unwrap();

        assert!(!read_core.get_can_write().unwrap());

        let mut read_lock = read_core.build().unwrap();
        let ProcessLockRead {
            core:
                ProcessLockCore {
                    run_id,
                    path,
                    lock: _,
                },
            process:
                ProcessInfoRead {
                    pid,
                    script_id,
                    raw_file_content: _,
                    file_content_start: _,
                },
        } = &read_lock;
        assert_eq!(RUN_ID, *run_id);
        assert_eq!(&file_path, path);
        assert_eq!(std::process::id(), *pid);
        assert_eq!(SCRIPT_ID, *script_id);
        assert!(read_lock.process.file_content().starts_with(SCRIPT_NAME));

        assert!(!read_lock.core.get_can_write().unwrap());
        drop(write_guard);
        assert!(read_lock.core.get_can_write().unwrap());
    }
    #[test]
    fn test_process_success() {
        const RUN_ID: i64 = 11;
        const SCRIPT_ID: i64 = 22;
        let file_path = crate::path::get_process_lock(RUN_ID).unwrap();

        let mut write_lock = ProcessLockWrite::new(RUN_ID, SCRIPT_ID, SCRIPT_NAME, &[]).unwrap();
        let new_read_lock = || {
            let read_core =
                ProcessLockRead::builder(file_path.clone(), &RUN_ID.to_string()).unwrap();
            read_core.build().unwrap()
        };

        let write_guard = write_lock.try_write_info().unwrap();
        drop(write_guard);

        let read_lock = new_read_lock();
        let res = read_lock.wait_write();
        assert!(matches!(res, Err(Error::ScriptError(_))));

        let read_lock = new_read_lock();
        let write_guard = write_lock.try_write_info().unwrap();
        ProcessLockWrite::mark_sucess(write_guard);

        read_lock.wait_write().expect("應該要成功");
    }
}
