use hyper_scripter::{fuzzy::*, my_env_logger, SEP};

type Str = &'static str;

fn unwrap_fuzz(target: Str, candidate: &[Str]) -> Vec<Str> {
    let _ = my_env_logger::try_init();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let res = rt.block_on(async {
        fuzz(target, candidate.iter().map(|t| *t), SEP)
            .await
            .unwrap()
            .unwrap()
    });
    match res {
        Multi {
            ans, mut others, ..
        } => {
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
    assert_vec!(
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

const HYPER_SCRIPTER: Str = "dir/hyper-scripter";
const SCRIPT: Str = "dir/script";

#[test]
fn test_fuzzy_6() {
    let v = vec![SCRIPT, HYPER_SCRIPTER];
    assert_vec!(unwrap_fuzz(SCRIPT, &v), vec![SCRIPT]);
}

const BUILD_HS: Str = "hs/build";
const UTIL_HISTORIAN: Str = "util/historian";
const UILH: Str = "uilh";
#[test]
fn test_fuzzy_7() {
    let v = vec![BUILD_HS, UTIL_HISTORIAN];
    assert_vec!(unwrap_fuzz(UILH, &v), vec![BUILD_HS]);
}

const AB: Str = "ab";
const ABC: Str = "abc";
const A: Str = "a";
#[test]
fn test_fuzzy_exact() {
    let v = vec![AB, ABC];
    assert_vec!(unwrap_fuzz(A, &v), vec![ABC, AB]);
    assert_vec!(unwrap_fuzz(AB, &v), vec![AB]);
}

const DOT: Str = ".";
const DOT_ONE: Str = ".1";
const ONE: Str = "1";
const TWELVE: Str = "12";
const A_SLASH_ONE: Str = "a/1";
const A_ONE: Str = "a1";
#[test]
fn test_fuzzy_anonymous() {
    let v = vec![DOT_ONE, TWELVE, A_ONE, A_SLASH_ONE];
    assert_vec!(unwrap_fuzz(ONE, &v), vec![DOT_ONE, A_SLASH_ONE, TWELVE]);

    let v = vec![DOT_ONE, TWELVE, A_ONE, ONE, A_SLASH_ONE];
    assert_vec!(unwrap_fuzz(ONE, &v), vec![ONE]);

    assert_vec!(unwrap_fuzz(DOT, &v), vec![DOT_ONE]);
}
