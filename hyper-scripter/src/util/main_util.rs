use crate::error::{Contextable, Error, RedundantOpt, Result};
use crate::path;
use crate::query::{self, EditQuery};
use crate::script::{IntoScriptName, ScriptInfo, ScriptName};
use crate::script_repo::{RepoEntry, ScriptRepo};
use crate::script_type::{iter_default_templates, ScriptType};
use crate::tag::{Tag, TagFilter};
use hyper_scripter_historian::Historian;
use std::path::{Path, PathBuf};

pub struct EditTagArgs {
    pub content: TagFilter,
    pub append_namespace: bool,

    pub explicit_tag: bool,
    pub explicit_filter: bool,
}

pub async fn mv(
    entry: &mut RepoEntry<'_>,
    new_name: Option<ScriptName>,
    ty: Option<ScriptType>,
    tags: Option<TagFilter>,
) -> Result {
    let og_path = path::open_script(&entry.name, &entry.ty, Some(true))?;
    if ty.is_some() || new_name.is_some() {
        let new_name = new_name.as_ref().unwrap_or(&entry.name);
        let new_ty = ty.as_ref().unwrap_or(&entry.ty);
        let new_path = path::open_script(new_name, new_ty, None)?;
        if new_path != og_path {
            log::debug!("改動腳本檔案：{:?} -> {:?}", og_path, new_path);
            if new_path.exists() {
                return Err(Error::PathExist(new_path).context("移動成既存腳本"));
            }
            super::mv(&og_path, &new_path)?;
        } else {
            log::debug!("相同的腳本檔案：{:?}，不做檔案處理", og_path);
        }
    }

    entry
        .update(|info| {
            if let Some(ty) = ty {
                info.ty = ty;
            }
            if let Some(name) = new_name {
                info.name = name;
            }
            if let Some(tags) = tags {
                // TODO: delete tag
                if tags.append {
                    log::debug!("附加上標籤：{:?}", tags);
                    tags.fill_allowed_map(&mut info.tags);
                } else {
                    log::debug!("設定標籤：{:?}", tags);
                    info.tags = tags.into_allowed_iter().collect();
                }
            }
            info.write();
        })
        .await
}
// XXX 到底幹嘛把新增和編輯的邏輯攪在一處呢…？
pub async fn edit_or_create(
    edit_query: EditQuery,
    script_repo: &'_ mut ScriptRepo,
    ty: Option<ScriptType>,
    tags: EditTagArgs,
) -> Result<(PathBuf, RepoEntry<'_>)> {
    let final_ty: ScriptType;
    let mut new_namespaces: Vec<Tag> = vec![];

    let (script_name, script_path) = if let EditQuery::Query(query) = edit_query {
        macro_rules! new_named {
            () => {{
                if tags.explicit_filter {
                    return Err(RedundantOpt::Filter.into());
                }
                final_ty = ty.unwrap_or_default();
                let name = query.into_script_name()?;
                if script_repo.get_hidden_mut(&name).is_some() {
                    log::error!("與被篩掉的腳本撞名");
                    return Err(Error::ScriptIsFiltered(name.to_string()));
                }
                log::debug!("打開新命名腳本：{:?}", name);
                if tags.append_namespace {
                    new_namespaces = name
                        .namespaces()
                        .iter()
                        .map(|s| s.parse())
                        .collect::<Result<Vec<Tag>>>()?;
                }

                let p = path::open_script(&name, &final_ty, None)
                    .context(format!("打開新命名腳本失敗：{:?}", name))?;
                if p.exists() {
                    log::warn!("編輯野生腳本！");
                }
                (name, p)
            }};
        }

        match query::do_script_query(&query, script_repo, false, None).await {
            Err(Error::DontFuzz) => new_named!(),
            Ok(None) => new_named!(),
            Ok(Some(entry)) => {
                if ty.is_some() {
                    return Err(RedundantOpt::Type.into());
                }
                if tags.explicit_tag {
                    return Err(RedundantOpt::Tag.into());
                }
                log::debug!("打開既有命名腳本：{:?}", entry.name);
                let p = path::open_script(&entry.name, &entry.ty, Some(true))
                    .context(format!("打開命名腳本失敗：{:?}", entry.name))?;
                // NOTE: 直接返回
                // FIXME: 一旦 NLL 進化就修掉這段雙重詢問
                // return Ok((p, entry));
                let n = entry.name.clone();
                return Ok((p, script_repo.get_mut(&n, true).unwrap()));
            }
            Err(e) => return Err(e),
        }
    } else {
        final_ty = ty.unwrap_or_default();
        log::debug!("打開新匿名腳本");
        path::open_new_anonymous(&final_ty).context("打開新匿名腳本失敗")?
    };

    log::info!("編輯 {:?}", script_name);

    // 這裡的 or_insert 其實永遠會發生，所以無需用閉包來傳
    let entry = script_repo
        .entry(&script_name)
        .or_insert(
            ScriptInfo::builder(
                0,
                script_name,
                final_ty,
                tags.content
                    .into_allowed_iter()
                    .chain(new_namespaces.into_iter()),
            )
            .build(),
        )
        .await?;

    Ok((script_path, entry))
}

pub async fn run_n_times(
    repeat: u64,
    dummy: bool,
    entry: &mut RepoEntry<'_>,
    mut args: &[String],
    historian: Historian,
    res: &mut Vec<Error>,
    previous_args: bool,
) -> Result {
    log::info!("執行 {:?}", entry.name);

    let previous_arg_vec: Vec<String>;
    if previous_args {
        if !args.is_empty() {
            return Err(RedundantOpt::CommandArgs.into());
        }
        match historian.last_args(entry.id).await? {
            None => return Err(Error::NoPreviousArgs),
            Some(arg_str) => {
                log::debug!("撈到前一次呼叫的參數 {}", arg_str);
                previous_arg_vec =
                    serde_json::from_str(&arg_str).context(format!("反序列失敗 {}", arg_str))?;
                args = &previous_arg_vec;
                // else 空字串當它是空陣列
            }
        }
    }

    let script_path = path::open_script(&entry.name, &entry.ty, Some(true))?;
    let content = super::read_file(&script_path)?;
    entry.update(|info| info.exec(content, args)).await?;

    if dummy {
        log::info!("--dummy 不用真的執行，提早退出");
        return Ok(());
    }

    for _ in 0..repeat {
        let run_res = super::run(
            &script_path,
            &*entry,
            &args,
            &entry.exec_time.as_ref().unwrap().data().unwrap().0,
        );
        let ret_code: i32;
        match run_res {
            Err(Error::ScriptError(code)) => {
                ret_code = code;
                res.push(run_res.unwrap_err());
            }
            Err(e) => return Err(e),
            Ok(_) => ret_code = 0,
        }
        entry.update(|info| info.exec_done(ret_code)).await?;
    }
    Ok(())
}

pub async fn load_utils(script_repo: &mut ScriptRepo) -> Result {
    let utils = hyper_scripter_util::get_all();
    for u in utils.into_iter() {
        log::info!("載入小工具 {}", u.name);
        let name = u.name.to_owned().into_script_name()?;
        if script_repo.get_mut(&name, true).is_some() {
            log::warn!("已存在的小工具 {:?}，跳過", name);
            continue;
        }
        let ty = u.ty.parse()?;
        let tags: Vec<Tag> = if u.is_hidden {
            vec!["util".parse().unwrap(), "hide".parse().unwrap()]
        } else {
            vec!["util".parse().unwrap()]
        };
        let p = path::open_script(&name, &ty, Some(false))?;
        let mut entry = script_repo
            .entry(&name)
            .or_insert(ScriptInfo::builder(0, name, ty, tags.into_iter()).build())
            .await?;
        super::prepare_script(&p, &*entry, true, &[u.content])?;
        entry.update(|info| info.write()).await?;
    }
    Ok(())
}

pub fn prepare_pre_run() -> Result {
    let p = path::get_home().join(path::HS_PRE_RUN);
    if !p.exists() {
        log::info!("寫入預執行腳本 {:?}", p);
        super::write_file(&p, ">&2 echo running $NAME $@")?;
    }
    Ok(())
}

pub fn load_templates() -> Result {
    for (ty, tmpl) in iter_default_templates() {
        let tmpl_path = path::get_template_path(&ty)?;
        if tmpl_path.exists() {
            continue;
        }
        super::write_file(&tmpl_path, tmpl)?;
    }
    Ok(())
}

use super::PrepareRespond;
pub async fn after_script(
    entry: &mut RepoEntry<'_>,
    path: &Path,
    prepare_resp: &PrepareRespond,
) -> Result<bool> {
    match prepare_resp {
        PrepareRespond::HasContent => {
            log::debug!("帶內容腳本，不執行後處理");
        }
        PrepareRespond::NoContent { is_new, time } => {
            let modified = super::file_modify_time(path)?;
            if time >= &modified {
                return Ok(if *is_new {
                    log::info!("新腳本未變動，應刪除之");
                    super::remove(path)?;
                    false
                } else {
                    log::info!("舊本未變動，不記錄事件");
                    true
                });
            }
        }
    }
    entry.update(|info| info.write()).await?;
    Ok(true)
}
