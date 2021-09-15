use env_logger;
use hyper_scripter::fuzzy::*;

type Str = &'static str;

fn unwrap_fuzz(target: Str, candidate: &[Str]) -> Vec<Str> {
    let _ = env_logger::try_init();
    let mut rt = tokio::runtime::Runtime::new().unwrap();
    let res = rt.block_on(async {
        fuzz(target, candidate.iter().map(|t| *t), "/")
            .await
            .unwrap()
            .unwrap()
    });
    match res {
        Multi { ans, mut others } => {
            others.push(ans);
            others
        }
        High(s) => {
            vec![s]
        }
        Low(s) => {
            // NOTE: 先不管這個 high 或 low 的問題
            vec![s]
        }
    }
}

macro_rules! assert_vec {
    ($v1:expr, $v2:expr) => {
        let mut v1 = $v1.clone();
        v1.sort();
        let mut v2 = $v2.clone();
        v2.sort();
        assert_eq!(v1, v2);
    };
}

const DISCORD_RUN: Str = "discord/run";
const DISCORD_DIR: Str = "discord/dir";
const DIR: Str = "dir";

const CB_RUN: Str = "cb/run";
const CROUCHING_DRAGON_RUN: Str = "crouching-dragon/run";
const RUN: Str = "run";

const UTIL_COMMIT: Str = "util/commit";
const FISH_CONFIG: Str = "config.fish";
const HS_COMMIT: Str = "hs/commit";
const CI: Str = "ci";

const REGRUN: Str = "regrun";
const REF: Str = "ref";
const RE: Str = "re";

const VCS_32: Str = "dir/vcs/32";
const VCS2: Str = "dir/vcs2";

#[test]
fn test_fuzzy_1() {
    assert_eq!(
        unwrap_fuzz(DIR, &vec![DISCORD_RUN, DISCORD_DIR]),
        vec![DISCORD_DIR]
    );
}

#[test]
fn test_fuzzy_2() {
    let v = vec![CB_RUN, CROUCHING_DRAGON_RUN];
    assert_vec!(unwrap_fuzz(RUN, &v), v);
}

#[test]
fn test_fuzzy_3() {
    let v = vec![HS_COMMIT, FISH_CONFIG, UTIL_COMMIT];
    assert_vec!(unwrap_fuzz(CI, &v), v);
}

#[test]
fn test_fuzzy_4() {
    let v = vec![REF, REGRUN];
    assert_vec!(unwrap_fuzz(RE, &v), v);
}

#[test]
fn test_fuzzy_5() {
    assert_vec!(unwrap_fuzz(VCS2, &vec![VCS2, VCS_32]), vec![VCS2]);
}

const HYPER_SCRIPTER: Str = "hyper-scripter";
const SCRIPT: Str = "script";
const GGSCRIPT: Str = "ggscript";

#[test]
fn test_fuzzy_6() {
    let v = vec![SCRIPT, HYPER_SCRIPTER, GGSCRIPT];
    assert_vec!(unwrap_fuzz(SCRIPT, &v), vec![HYPER_SCRIPTER, SCRIPT]);
}
