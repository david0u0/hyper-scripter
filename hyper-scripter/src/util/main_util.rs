use crate::error::{Contextable, Error, Result};
use crate::path;
use crate::query::{self, do_script_query_strict_with_missing, EditQuery, ScriptQuery};
use crate::script::{IntoScriptName, ScriptInfo, ScriptName};
use crate::script_repo::{ScriptRepo, ScriptRepoEntry};
use crate::script_type::ScriptType;
use crate::tag::{Tag, TagControlFlow};
use hyper_scripter_historian::{Event, EventData, Historian};
use std::path::PathBuf;

pub struct EditTagArgs {
    pub content: TagControlFlow,
    pub change_existing: bool,
    pub append_namespace: bool,
}

pub async fn mv<'b>(
    entry: &mut ScriptRepoEntry<'b>,
    new_name: Option<ScriptName>,
    ty: Option<ScriptType>,
    tags: Option<TagControlFlow>,
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
                if tags.append {
                    log::debug!("附加上標籤：{:?}", tags);
                    info.tags.extend(tags.into_allowed_iter());
                } else {
                    log::debug!("設定標籤：{:?}", tags);
                    info.tags = tags.into_allowed_iter().collect();
                }
            }
            info.write();
        })
        .await
}
pub async fn edit_or_create<'b>(
    edit_query: EditQuery,
    script_repo: &'b mut ScriptRepo,
    ty: Option<ScriptType>,
    tags: EditTagArgs,
) -> Result<(PathBuf, ScriptRepoEntry<'b>)> {
    let final_ty: ScriptType;
    let mut new_namespaces: Vec<Tag> = vec![];

    let (script_name, script_path) = if let EditQuery::Query(query) = edit_query {
        macro_rules! new_named {
            () => {{
                final_ty = ty.unwrap_or_default();
                let name = query.into_script_name()?;
                if script_repo.get_hidden_mut(&name).is_some() {
                    log::error!("與被篩掉的腳本撞名");
                    return Err(Error::ScriptExist(name.to_string()));
                }
                log::debug!("打開新命名腳本：{:?}", name);
                if tags.append_namespace {
                    new_namespaces = name
                        .namespaces()
                        .iter()
                        .map(|s| s.parse())
                        .collect::<Result<Vec<Tag>>>()?;
                }

                let p = path::open_script(&name, &final_ty, Some(false))
                    .context(format!("打開新命名腳本失敗：{:?}", name))?;
                (name, p)
            }};
        }

        match query::do_script_query(&query, script_repo) {
            Err(Error::DontFuzz) => new_named!(),
            Ok(None) => new_named!(),
            Ok(Some(mut entry)) => {
                if let Some(ty) = ty {
                    log::warn!("已存在的腳本無需再指定類型");
                    if ty != entry.ty {
                        return Err(Error::CategoryMismatch {
                            expect: ty,
                            actual: entry.ty.clone(),
                        });
                    }
                }
                if tags.change_existing {
                    mv(&mut entry, None, None, Some(tags.content)).await?;
                }
                log::debug!("打開既有命名腳本：{:?}", entry.name);
                let p = path::open_script(&entry.name, &entry.ty, Some(true))
                    .context(format!("打開命名腳本失敗：{:?}", entry.name))?;
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
    n: u64,
    script_query: &ScriptQuery,
    script_repo: &mut ScriptRepo,
    args: &[String],
    historian: Historian,
    res: &mut Vec<Error>,
) -> Result {
    let mut entry = do_script_query_strict_with_missing(&script_query, script_repo).await?;
    log::info!("執行 {:?}", entry.name);
    {
        let exe = std::env::current_exe()?;
        let exe = exe.to_string_lossy();
        log::debug!("將 hs 執行檔的確切位置 {} 記錄起來", exe);
        super::write_file(&path::get_home().join(path::HS_EXECUTABLE_INFO_PATH), &exe)?;
    }
    let script_path = path::open_script(&entry.name, &entry.ty, Some(true))?;
    let content = super::read_file(&script_path)?;
    entry.update(|info| info.exec(content)).await?;
    for _ in 0..n {
        let run_res = super::run(
            &script_path,
            &*entry,
            &args,
            entry.exec_time.as_ref().unwrap().data().unwrap(),
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
        historian
            .record(&Event {
                data: EventData::ExecDone(ret_code),
                script_id: entry.id,
            })
            .await?;
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
        let ty = u.category.parse()?;
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
        super::prepare_script(&p, &*entry, true, Some(u.content))?;
        entry.update(|info| info.write()).await?;
    }
    Ok(())
}
