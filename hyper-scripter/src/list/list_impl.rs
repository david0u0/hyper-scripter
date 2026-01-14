use super::{
    exec_time_str, extract_help, get_screen_width, style, style_name,
    table_lib::{Cell, Collumn, Table},
    time_fmt, tree, DisplayStyle, Grid, Grouping, IdentTemplate, ListOptions, LONG_LATEST_TXT,
    SHORT_LATEST_TXT,
};
use crate::error::Result;
use crate::query::{do_list_query, ListQuery};
use crate::script::ScriptInfo;
use crate::script_repo::{ScriptRepo, Visibility};
use crate::tag::Tag;
use crate::util::{get_display_type, DisplayType};
use fxhash::FxHashMap as HashMap;
use handlebars::{Context, Handlebars, RenderContext, Renderable, Template};
use serde::Serialize;
use std::borrow::Cow;
use std::cmp::Reverse;
use std::hash::Hash;
use std::io::Write;

type ListOptionWithOutput = ListOptions<Table, Grid>;

fn render_general_ident(
    format: &Template,
    name: &str,
    ty: &DisplayType,
    script: &ScriptInfo,
) -> Result<String> {
    #[derive(Serialize)]
    pub struct TmplVal<'a> {
        id: i64,
        name: &'a str,
        file: Cow<'a, str>,
        ty: Cow<'a, str>,
    }
    let file = script.file_path_fallback();
    let tmpl_val = TmplVal {
        id: script.id,
        file: file.to_string_lossy(),
        ty: ty.display(),
        name,
    };

    let reg = Handlebars::new();
    let res = format.renders(
        &reg,
        &Context::wraps(tmpl_val)?,
        &mut RenderContext::new(None),
    )?;
    Ok(res)
}

pub fn ident_string(
    format: &IdentTemplate,
    name: &str,
    ty: &DisplayType,
    script: &ScriptInfo,
) -> Result<String> {
    let res = match format {
        IdentTemplate::General(t) => return render_general_ident(t, name, ty, script),
        IdentTemplate::Classic => format!("{}({})", name, ty.display()),
        IdentTemplate::Name => name.to_owned(),
        IdentTemplate::File => script.file_path_fallback().to_string_lossy().to_string(),
        IdentTemplate::ID => script.id.to_string(),
    };
    Ok(res)
}

#[derive(PartialEq, Eq, Hash)]
struct TagsKey(Vec<Tag>);
impl std::fmt::Display for TagsKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            write!(f, "(no tag)")?;
            return Ok(());
        }
        write!(f, "[")?;
        let mut first = true;
        for tag in &self.0 {
            if !first {
                write!(f, " ")?;
            }
            first = false;
            write!(f, "#{}", AsRef::<str>::as_ref(tag))?;
        }
        write!(f, "]")?;
        Ok(())
    }
}
impl TagsKey {
    fn new(tags: impl Iterator<Item = Tag>) -> Self {
        let mut tags: Vec<_> = tags.collect();
        tags.sort();
        TagsKey(tags)
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn sort_scripts(v: &mut Vec<&ScriptInfo>) {
    v.sort_by_key(|s| Reverse(s.last_time()));
}

fn convert_opt<T>(opt: ListOptions, t: T) -> ListOptions<Table, T> {
    ListOptions {
        display_style: match opt.display_style {
            DisplayStyle::Short(format, _) => DisplayStyle::Short(format, t),
            DisplayStyle::Long(_) => {
                time_fmt::init();
                let mut table = Table::new(gen_title());
                table.set_width(get_screen_width());
                table.set_plain(opt.plain);
                DisplayStyle::Long(table)
            }
        },
        grouping: opt.grouping,
        plain: opt.plain,
        limit: opt.limit,
    }
}
fn extract_table<U>(opt: ListOptions<Table, U>) -> Option<Table> {
    match opt.display_style {
        DisplayStyle::Short(..) => None,
        DisplayStyle::Long(table) => Some(table),
    }
}
pub fn fmt_meta(
    script: &ScriptInfo,
    is_latest: bool,
    opt: &mut ListOptionWithOutput,
) -> Result<()> {
    let ty = get_display_type(&script.ty);
    let color = ty.color();
    match &mut opt.display_style {
        DisplayStyle::Long(table) => {
            let (name_txt, name_width) = style_name(
                opt.plain,
                is_latest,
                LONG_LATEST_TXT,
                color,
                &script.name.key(),
            )?;
            let ty = ty.display();
            let ty_width = ty.len();
            let ty_txt = style(opt.plain, ty, |s| s.color(color).bold().done());

            let help_msg = extract_help(script);

            let row = vec![
                Cell::new_with_len(name_txt, name_width),
                Cell::new_with_len(ty_txt.to_string(), ty_width),
                Cell::new(time_fmt::fmt(&script.write_time).to_string()),
                Cell::new(exec_time_str(script).to_string()),
                Cell::new(help_msg),
            ];
            table.add_row(row);
        }
        DisplayStyle::Short(format, grid) => {
            let ident = ident_string(format, &script.name.to_string(), &ty, script)?;
            let (ident, width) = style_name(opt.plain, is_latest, SHORT_LATEST_TXT, color, &ident)?;
            grid.add(ident, width);
        }
    }
    Ok(())
}

enum ScriptsEither<'a, I> {
    Iter(I),
    V(Vec<&'a ScriptInfo>),
}
impl<'a, I: ExactSizeIterator<Item = &'a ScriptInfo>> ScriptsEither<'a, I> {
    fn new(iter: I, limit: Option<usize>) -> Self {
        if let Some(limit) = limit {
            let mut v: Vec<_> = iter.collect();
            sort_scripts(&mut v);
            v.truncate(limit);
            Self::V(v)
        } else {
            Self::Iter(iter)
        }
    }
    fn collect(self) -> Vec<&'a ScriptInfo> {
        match self {
            Self::Iter(iter) => iter.collect(),
            Self::V(v) => v,
        }
    }
    fn len(&self) -> usize {
        match self {
            Self::Iter(iter) => iter.len(),
            Self::V(v) => v.len(),
        }
    }
    fn sorted(&self) -> bool {
        matches!(self, Self::V(..))
    }
    fn for_each<F: FnMut(&'a ScriptInfo)>(self, mut f: F) {
        match self {
            Self::Iter(iter) => {
                for s in iter {
                    f(s);
                }
            }
            Self::V(v) => {
                for s in v.into_iter() {
                    f(s);
                }
            }
        }
    }
}

fn gen_title() -> Vec<Collumn> {
    vec![
        Collumn::new_fixed("Script"),
        Collumn::new_fixed("Type"),
        Collumn::new("Write"),
        Collumn::new("Execute"),
        Collumn::new("Help Message"),
    ]
}

pub async fn fmt_list<W: Write>(
    w: &mut W,
    script_repo: &mut ScriptRepo,
    opt: ListOptions,
    queries: Vec<ListQuery>,
) -> Result<()> {
    if !opt.plain && script_repo.time_hidden_count > 0 {
        write!(
            w,
            "{} scripts ignored due to time filter: {}\n",
            script_repo.time_hidden_count, script_repo.recent_filter
        )?;
    }

    let latest_script_id = script_repo
        .latest_mut(1, Visibility::Normal)
        .map_or(-1, |s| s.id);

    let scripts_iter = do_list_query(script_repo, queries)
        .await?
        .into_iter()
        .map(|e| &*e.into_inner());
    let scripts_either = ScriptsEither::new(scripts_iter, opt.limit.map(|l| l.get()));
    let sorted = scripts_either.sorted();

    let final_table: Option<Table>;
    match opt.grouping {
        Grouping::None => {
            let mut opt = convert_opt(opt, Grid::new(scripts_either.len()));
            let scripts = scripts_either.collect();
            fmt_group(w, scripts, sorted, latest_script_id, &mut opt)?;
            final_table = extract_table(opt);
        }
        Grouping::Tree => {
            let mut opt = convert_opt(opt, &mut *w);
            let scripts = scripts_either.collect();
            tree::fmt(scripts, latest_script_id, &mut opt)?;
            final_table = extract_table(opt);
        }
        Grouping::Tag => {
            let mut opt = convert_opt(opt, Grid::new(scripts_either.len()));
            let mut script_map: HashMap<TagsKey, Vec<&ScriptInfo>> = HashMap::default();
            scripts_either.for_each(|script| {
                let key = TagsKey::new(script.tags.iter().cloned());
                let v = script_map.entry(key).or_default();
                v.push(script);
            });

            let mut scripts: Vec<_> = script_map
                .into_iter()
                .map(|(k, v)| {
                    // NOTE: 以群組中執行次數的最大值排序, 無標籤永遠在上
                    let sort_key = if k.is_empty() {
                        None
                    } else {
                        v.iter()
                            .map(|s| {
                                if s.exec_time.is_none() {
                                    0
                                } else {
                                    s.exec_count
                                }
                            })
                            .max()
                    };
                    (sort_key, k, v)
                })
                .collect();

            scripts.sort_by_key(|(sort_key, _, _)| *sort_key);

            for (_, tags, scripts) in scripts.into_iter() {
                if !opt.grouping.is_none() {
                    let tags_txt = style(opt.plain, tags, |s| s.dimmed().italic().done());
                    match &mut opt.display_style {
                        DisplayStyle::Long(table) => {
                            table.add_row(vec![Cell::new_with_len(tags_txt.to_string(), 0)]);
                        }
                        DisplayStyle::Short(_, _) => {
                            writeln!(w, "{}", tags_txt)?;
                        }
                    }
                }
                fmt_group(w, scripts, sorted, latest_script_id, &mut opt)?;
            }
            final_table = extract_table(opt);
        }
    }
    if let Some(mut table) = final_table {
        write!(w, "{}", table.display())?;
        log::debug!("tree table: {:?}", table);
    }
    Ok(())
}

fn fmt_group<W: Write>(
    w: &mut W,
    mut scripts: Vec<&ScriptInfo>,
    sorted: bool,
    latest_script_id: i64,
    opt: &mut ListOptionWithOutput,
) -> Result<()> {
    if !sorted {
        sort_scripts(&mut scripts);
    }
    for script in scripts.into_iter() {
        let is_latest = script.id == latest_script_id;
        fmt_meta(script, is_latest, opt)?;
    }
    match &mut opt.display_style {
        DisplayStyle::Short(_, grid) => {
            let grid_display = grid.fit_into_screen();
            write!(w, "{}", grid_display)?;
            drop(grid_display);
            grid.clear();
        }
        _ => (),
    }
    Ok(())
}
