use hyper_scripter::fuzzy::*;

type Str = &'static str;

fn unwrap_fuzz(target: Str, candidate: &[Str]) -> Vec<Str> {
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
            others.sort();
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

const VCS_32: Str = "dir/vcs32";
const VCS2: Str = "dir/vcs2";

#[test]
fn test_fuzzy_1() {
    assert_eq!(
        unwrap_fuzz(DIR, &[DISCORD_RUN, DISCORD_DIR]),
        vec![DISCORD_DIR]
    );
}

#[test]
fn test_fuzzy_2() {
    let v = vec![CB_RUN, CROUCHING_DRAGON_RUN];
    assert_eq!(unwrap_fuzz(RUN, &v), v);
}

#[test]
fn test_fuzzy_3() {
    let v = vec![HS_COMMIT, FISH_CONFIG, UTIL_COMMIT];
    assert_eq!(unwrap_fuzz(CI, &v), v);
}

#[test]
fn test_fuzzy_4() {
    let v = vec![REF, REGRUN];
    assert_eq!(unwrap_fuzz(RE, &v), v);
}

#[test]
fn test_fuzzy_5() {
    assert_eq!(unwrap_fuzz(VCS2, &[VCS2, VCS_32]), vec![VCS2]);
}
