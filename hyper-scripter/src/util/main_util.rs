use crate::error::{Contextable, Error, Result};
use crate::path;
use crate::query::{self, EditQuery};
use crate::script::{IntoScriptName, ScriptInfo, ScriptName};
use crate::script_repo::{ScriptRepo, ScriptRepoEntry};
use crate::script_type::ScriptType;
use crate::tag::{Tag, TagControlFlow};
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
    // FIXME: 應該要允許 mv 成既存的名字
    let og_script = path::open_script(&entry.name, &entry.ty, Some(true))?;
    if ty.is_some() || new_name.is_some() {
        let new_script = path::open_script(
            new_name.as_ref().unwrap_or(&entry.name),
            ty.as_ref().unwrap_or(&entry.ty),
            Some(false),
        )?;
        super::mv(&og_script, &new_script)?;
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
