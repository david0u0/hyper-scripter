use crate::config::Config;
use crate::error::{
    Contextable, DisplayError, DisplayResult, Error, FormatCode::ScriptName as ScriptNameCode,
    Result,
};
use crate::script_time::ScriptTime;
use crate::script_type::ScriptType;
use crate::tag::{Tag, TagSelector};
use crate::util::illegal_name;
use chrono::NaiveDateTime;
use fxhash::FxHashSet as HashSet;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::Write;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::str::FromStr;

pub const ANONYMOUS: &str = ".anonymous";

macro_rules! max {
    ($x:expr) => ( $x );
    ($x:expr, $($xs:expr),+) => {
        {
            use std::cmp::max;
            max($x, max!( $($xs),+ ))
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Deref, Display, Hash)]
#[display(fmt = "{}", inner)]
pub struct ConcreteScriptName {
    #[deref]
    inner: String,
}
impl ConcreteScriptName {
    fn valid(s: &str) -> Result {
        // FIXME: 好好想想什麼樣的腳本名可行，並補上單元測試
        for s in s.split('/') {
            if illegal_name(s) {
                return ScriptNameCode.to_res(s.to_owned());
            }
        }
        Ok(())
    }
    pub fn new(s: String) -> Result<Self> {
        Self::valid(&s)?;
        Ok(ConcreteScriptName { inner: s })
    }
    pub fn new_id(id: u32) -> Self {
        ConcreteScriptName {
            inner: id.to_string(),
        }
    }
    fn new_unchecked(s: String) -> Self {
        ConcreteScriptName { inner: s }
    }
    fn stem_inner(&self) -> &str {
        if let Some((_, stem)) = self.inner.rsplit_once('/') {
            stem
        } else {
            &self.inner
        }
    }
    pub fn stem(&self) -> ConcreteScriptName {
        Self::new_unchecked(self.stem_inner().to_owned())
    }
    pub fn join(&mut self, other: &ConcreteScriptName) {
        write!(&mut self.inner, "/{}", other.stem_inner()).unwrap();
    }
    pub fn join_id(&mut self, other: u32) {
        write!(&mut self.inner, "/{}", other).unwrap();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScriptName {
    Anonymous(u32),
    Named(ConcreteScriptName),
}
/// ```
/// use hyper_scripter::script::*;
///
/// let name: ScriptName = ".42".parse().unwrap();
/// assert_eq!(name, ScriptName::Anonymous(42));
///
/// let name: ScriptName = "name".parse().unwrap();
/// assert_eq!(name.to_string(), "name");
///
/// let res: Result<ScriptName, _> = ".0".parse();
/// res.unwrap_err();
/// ```
impl FromStr for ScriptName {
    type Err = DisplayError;
    fn from_str(s: &str) -> DisplayResult<Self> {
        let n = s.to_owned().into_script_name()?;
        Ok(n)
    }
}
impl ScriptName {
    #[inline]
    pub fn valid(
        mut s: &str,
        allow_endwith_slash: bool,
        allow_dot: bool,
        check: bool,
    ) -> Result<Option<u32>> {
        log::debug!("檢查腳本名：{}", s);

        if s.starts_with('.') {
            if s.len() == 1 && allow_dot {
                log::info!("特殊規則：允許單一個`.`");
                return Ok(None); // NOTE: 讓匿名腳本可以直接用 `.` 來搜
            }
            match s[1..].parse::<std::num::NonZeroU32>() {
                Ok(id) => Ok(Some(id.get())),
                Err(e) => ScriptNameCode.to_res(s.to_owned()).context(e),
            }
        } else if check {
            if s.ends_with('/') && allow_endwith_slash {
                log::info!("特殊規則：允許以`/`結尾");
                s = &s[..s.len() - 1]; // NOTE: 有了補全，很容易補出帶著`/`結尾的指令，放寬標準吧
            }
            ConcreteScriptName::valid(&s)?;
            Ok(None)
        } else {
            Ok(None)
        }
    }
    pub fn namespaces(&self) -> Vec<&'_ str> {
        match self {
            ScriptName::Anonymous(_) => vec![],
            ScriptName::Named(s) => {
                let mut v: Vec<_> = s.split('/').collect();
                v.pop();
                v
            }
        }
    }
    pub fn is_anonymous(&self) -> bool {
        log::debug!("判斷是否為匿名：{:?}", self);
        matches!(self, ScriptName::Anonymous(_))
    }
    pub fn key(&self) -> Cow<'_, str> {
        match self {
            ScriptName::Anonymous(id) => Cow::Owned(format!(".{}", id)),
            ScriptName::Named(s) => Cow::Borrowed(s),
        }
    }
    /// 回傳值是相對於 HS_HOME 的路徑
    pub fn to_file_path(&self, ty: &ScriptType) -> Result<PathBuf> {
        self.to_file_path_inner(ty, false).map(|t| t.0)
    }
    /// 回傳值是相對於 HS_HOME 的路徑，對未知的類別直接用類別名作擴展名
    pub fn to_file_path_fallback(&self, ty: &ScriptType) -> (PathBuf, Option<Error>) {
        self.to_file_path_inner(ty, true).unwrap()
    }
    fn to_file_path_inner(
        &self,
        ty: &ScriptType,
        fallback: bool,
    ) -> Result<(PathBuf, Option<Error>)> {
        fn add_ext(
            name: &mut String,
            ty: &ScriptType,
            fallback: bool,
            err: &mut Option<Error>,
        ) -> Result {
            let ext = match Config::get().get_script_conf(ty) {
                Err(e) => {
                    if !fallback {
                        return Err(e);
                    }
                    log::warn!(
                        "取腳本路徑時找不到類別設定：{}，直接把類別名當擴展名試試",
                        e,
                    );
                    *err = Some(e);
                    Some(ty.as_ref())
                }
                Ok(c) => c.ext.as_ref().map(|s| s.as_ref()),
            };
            if let Some(ext) = ext {
                write!(name, ".{}", ext).unwrap();
            }
            Ok(())
        }
        let mut err = None;
        match self {
            ScriptName::Anonymous(id) => {
                let mut file_name = id.to_string();
                let dir: PathBuf = ANONYMOUS.into();
                add_ext(&mut file_name, ty, fallback, &mut err)?;
                Ok((dir.join(file_name), err))
            }
            ScriptName::Named(name) => {
                let mut file_name = name.to_string();
                add_ext(&mut file_name, ty, fallback, &mut err)?;
                Ok((file_name.into(), err))
            }
        }
    }
}
impl PartialOrd for ScriptName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (ScriptName::Named(n1), ScriptName::Named(n2)) => n1.inner.partial_cmp(&n2.inner),
            (ScriptName::Anonymous(i1), ScriptName::Anonymous(i2)) => i1.partial_cmp(i2),
            (ScriptName::Named(_), ScriptName::Anonymous(_)) => Some(Ordering::Less),
            (ScriptName::Anonymous(_), ScriptName::Named(_)) => Some(Ordering::Greater),
        }
    }
}
impl Ord for ScriptName {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl std::fmt::Display for ScriptName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key())
    }
}

#[derive(Debug, Clone)]
pub struct TimelessScriptInfo {
    pub changed: bool,
    pub id: i64,
    pub hash: i64,
    pub name: ScriptName,
    pub tags: HashSet<Tag>,
    pub ty: ScriptType,
    pub created_time: ScriptTime,
}
#[derive(Debug, Deref, Clone)]
pub struct ScriptInfo {
    pub humble_time: Option<NaiveDateTime>,
    pub read_time: ScriptTime,
    pub write_time: ScriptTime,
    pub neglect_time: Option<ScriptTime>,
    /// (content, args, env_record, dir)
    pub exec_time: Option<ScriptTime<(String, String, String, Option<PathBuf>)>>,
    /// (return code, main event id)
    pub exec_done_time: Option<ScriptTime<(i32, i64)>>,
    pub exec_count: u64,
    #[deref]
    /// 用來區隔「時間資料」和「其它元資料」，並偵測其它元資料的修改
    timeless_info: TimelessScriptInfo,
}
impl DerefMut for ScriptInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.timeless_info.changed = true;
        &mut self.timeless_info
    }
}

fn map<T: Deref<Target = NaiveDateTime>>(time: &Option<T>) -> NaiveDateTime {
    match time {
        Some(time) => **time,
        _ => Default::default(),
    }
}
impl ScriptInfo {
    pub fn set_id(&mut self, id: i64) {
        assert_eq!(self.id, 0, "只有 id=0（代表新腳本）時可以設定 id");
        self.timeless_info.id = id;
    }
    pub fn append_tags(&mut self, tags: TagSelector) {
        if tags.append {
            log::debug!("附加上標籤：{:?}", tags);
            tags.fill_allowed_map(&mut self.tags);
        } else {
            log::debug!("設定標籤：{:?}", tags);
            self.tags = tags.into_allowed_iter().collect();
        }
    }
    pub fn cp(&self, new_name: ScriptName) -> Self {
        let builder = ScriptInfo::builder(
            0,
            self.hash,
            new_name,
            self.ty.clone(),
            self.tags.iter().cloned(),
        );
        builder.build()
    }
    /// `major time` 即不包含 `read` 事件的時間，但包含 `humble`
    pub fn last_major_time(&self) -> NaiveDateTime {
        max!(
            *self.write_time,
            map(&self.humble_time.as_ref()),
            map(&self.exec_time),
            map(&self.exec_done_time)
        )
    }
    /// 不包含 `humble`
    pub fn last_time(&self) -> NaiveDateTime {
        max!(
            *self.read_time,
            *self.write_time,
            map(&self.exec_time),
            map(&self.exec_done_time)
        )
    }
    pub fn file_path_fallback(&self) -> PathBuf {
        self.name.to_file_path_fallback(&self.ty).0
    }
    pub fn read(&mut self) {
        self.read_time = ScriptTime::now(());
    }
    pub fn write(&mut self) {
        let now = ScriptTime::now(());
        self.read_time = now.clone();
        self.write_time = now;
    }
    pub fn exec(
        &mut self,
        content: String,
        args: &[String],
        env_record: String,
        dir: Option<PathBuf>,
    ) {
        log::trace!("{:?} 執行內容為 {}", self, content);
        let args_ser = serde_json::to_string(args).unwrap();
        self.exec_time = Some(ScriptTime::now((content, args_ser, env_record, dir)));
        // NOTE: no readtime, otherwise it will be hard to tell what event was caused by what operation.
        self.exec_count += 1;
    }
    pub fn exec_done(&mut self, code: i32, main_event_id: i64) {
        log::trace!("{:?} 執行結果為 {}", self, code);
        self.exec_done_time = Some(ScriptTime::now((code, main_event_id)));
    }
    pub fn neglect(&mut self) {
        self.neglect_time = Some(ScriptTime::now(()))
    }
    pub fn builder(
        id: i64,
        hash: i64,
        name: ScriptName,
        ty: ScriptType,
        tags: impl Iterator<Item = Tag>,
    ) -> ScriptBuilder {
        ScriptBuilder {
            id,
            hash,
            name,
            ty,
            tags: tags.collect(),
            read_time: None,
            created_time: None,
            exec_time: None,
            write_time: None,
            exec_done_time: None,
            neglect_time: None,
            humble_time: None,
            exec_count: 0,
        }
    }
}

pub trait IntoScriptName {
    fn into_script_name(self) -> Result<ScriptName>;
    fn into_script_name_unchecked(self) -> Result<ScriptName>
    where
        Self: Sized,
    {
        self.into_script_name()
    }
}

impl IntoScriptName for u32 {
    fn into_script_name(self) -> Result<ScriptName> {
        Ok(ScriptName::Anonymous(self))
    }
}
impl IntoScriptName for ConcreteScriptName {
    fn into_script_name(self) -> Result<ScriptName> {
        Ok(ScriptName::Named(self))
    }
}
#[inline]
fn string_into_script_name(s: String, check: bool) -> Result<ScriptName> {
    log::debug!("解析腳本名：{} {}", s, check);
    if let Some(id) = ScriptName::valid(&s, false, false, check)? {
        id.into_script_name()
    } else {
        Ok(ScriptName::Named(ConcreteScriptName::new_unchecked(s))) // NOTE: already checked by `ScriptName::valid`
    }
}
impl IntoScriptName for String {
    fn into_script_name(self) -> Result<ScriptName> {
        string_into_script_name(self, true)
    }
    fn into_script_name_unchecked(self) -> Result<ScriptName> {
        string_into_script_name(self, false)
    }
}
impl IntoScriptName for ScriptName {
    fn into_script_name(self) -> Result<ScriptName> {
        Ok(self)
    }
}

#[derive(Debug)]
pub struct ScriptBuilder {
    pub name: ScriptName,
    read_time: Option<NaiveDateTime>,
    created_time: Option<NaiveDateTime>,
    write_time: Option<NaiveDateTime>,
    exec_time: Option<NaiveDateTime>,
    neglect_time: Option<NaiveDateTime>,
    humble_time: Option<NaiveDateTime>,
    exec_done_time: Option<NaiveDateTime>,
    exec_count: u64,

    hash: i64,
    id: i64,
    tags: HashSet<Tag>,
    ty: ScriptType,
}

impl ScriptBuilder {
    pub fn exec_count(&mut self, count: u64) -> &mut Self {
        self.exec_count = count;
        self
    }
    pub fn exec_time(&mut self, time: NaiveDateTime) -> &mut Self {
        self.exec_time = Some(time);
        self
    }
    pub fn exec_done_time(&mut self, time: NaiveDateTime) -> &mut Self {
        self.exec_done_time = Some(time);
        self
    }
    pub fn read_time(&mut self, time: NaiveDateTime) -> &mut Self {
        self.read_time = Some(time);
        self
    }
    pub fn write_time(&mut self, time: NaiveDateTime) -> &mut Self {
        self.write_time = Some(time);
        self
    }
    pub fn neglect_time(&mut self, time: NaiveDateTime) -> &mut Self {
        self.neglect_time = Some(time);
        self
    }
    pub fn humble_time(&mut self, time: NaiveDateTime) -> &mut Self {
        self.humble_time = Some(time);
        self
    }
    pub fn created_time(&mut self, time: NaiveDateTime) -> &mut Self {
        self.created_time = Some(time);
        self
    }
    pub fn build(self) -> ScriptInfo {
        let created_time = ScriptTime::new_or(self.created_time, ScriptTime::now(()));
        ScriptInfo {
            write_time: ScriptTime::new_or(self.write_time, created_time),
            read_time: ScriptTime::new_or(self.read_time, created_time),
            exec_time: self.exec_time.map(ScriptTime::new),
            exec_done_time: self.exec_done_time.map(ScriptTime::new),
            neglect_time: self.neglect_time.map(ScriptTime::new),
            humble_time: self.humble_time,
            exec_count: self.exec_count,
            timeless_info: TimelessScriptInfo {
                changed: false,
                id: self.id,
                hash: self.hash,
                name: self.name,
                ty: self.ty,
                tags: self.tags,
                created_time,
            },
        }
    }
}
