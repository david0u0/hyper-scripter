#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]

use criterion::{black_box, Bencher, Criterion};
use criterion_macro::criterion;
use hyper_scripter::{fuzzy::*, main_inner::main_with_args};
use rand::{rngs::StdRng, seq::index::sample, Rng, SeedableRng};
use std::borrow::Cow;

#[allow(dead_code)]
#[path = "../tests/tool.rs"]
mod tool;
use tool::{get_home, setup};

fn split_args(s: &str) -> Vec<String> {
    let home = get_home();
    std::iter::once("hs-bench")
        .chain(tool::split_args(s, &home))
        .map(|s| s.to_string())
        .collect()
}

const LONG: usize = 20;
const SHORT: std::ops::Range<usize> = 5..15;
fn gen_name(rng: &mut StdRng) -> String {
    const CHARSET: &[u8] = b"///_ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                    abcdefghijklmnopqrstuvwxyz\
                                    0123456789";
    loop {
        let s: String = (0..LONG)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();

        if s.starts_with("/") || s.ends_with("/") {
            continue;
        }
        if s.find("//").is_some() {
            continue;
        }
        return s;
    }
}
fn sample_name(rng: &mut StdRng, name: &str) -> String {
    let mut ret = "".to_owned();
    let len = rng.gen_range(SHORT);
    let mut idx_sample: Vec<_> = sample(rng, LONG, len).iter().collect();
    idx_sample.sort();
    for idx in idx_sample.into_iter() {
        ret.push(name.chars().nth(idx).unwrap());
    }
    ret
}

#[criterion]
fn bench_fuzz(c: &mut Criterion) {
    let _ = env_logger::try_init();

    struct MyStr<'a>(&'a str);
    impl<'a> FuzzKey for MyStr<'a> {
        fn fuzz_key(&self) -> Cow<'a, str> {
            Cow::Borrowed(self.0)
        }
    }

    let mut rng = StdRng::seed_from_u64(42);
    const CASE_COUNT: usize = 999;

    let mut names = vec![];
    let mut shorts = vec![];
    for _ in 0..CASE_COUNT {
        let name = gen_name(&mut rng);
        let short = sample_name(&mut rng, &name);
        names.push(name);
        shorts.push(short);
    }

    let mut rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("fuzzy_func", |b| {
        b.iter(|| {
            rt.block_on(async {
                for short in shorts.iter() {
                    let names = names.iter().map(|s| MyStr(s.as_ref()));
                    let res = fuzz(short, names).await.unwrap();
                    black_box(res);
                }
            });
        });
    });
}

struct TestDate {
    data: Vec<(String, [i8; 3])>,
}
impl TestDate {
    fn new(count: usize, rng: &mut StdRng) -> Self {
        let mut data = vec![];
        for _ in 0..count {
            let name = gen_name(rng);
            data.push((name, gen_tag_arr(rng, 0, 1)));
        }
        TestDate { data }
    }
}
fn gen_tag_arr(rng: &mut StdRng, min: i8, max: i8) -> [i8; 3] {
    let mut tags = [0; 3];
    for j in 0..3 {
        tags[j] = rng.gen_range(min..=max);
    }
    tags
}
fn gen_tag_string(a: &[i8; 3]) -> String {
    let mut v = vec![];
    for (i, &u) in a.iter().enumerate() {
        match u {
            1 => v.push(format!("tag{}", i)),
            -1 => v.push(format!("^tag{}", i)),
            _ => (),
        }
    }
    if v.is_empty() {
        "all".to_owned()
    } else {
        v.join(",")
    }
}
fn gen_tag_filter_string(rng: &mut StdRng, mut a: [i8; 3]) -> String {
    for i in 0..3 {
        let should_messup = rng.gen_bool(0.5);
        if should_messup {
            a[i] = rng.gen_range(-1..=1);
        }
    }
    gen_tag_string(&a)
}
fn run_bench_with<F>(b: &mut Bencher, case_count: usize, epoch: usize, mut gen_arg: F)
where
    F: FnMut(&mut StdRng, &str, &[i8; 3]) -> String,
{
    // {case_count} scripts, with random tags from [tag0, tag1, tag2] (2^3 posible combinations)
    // Run {epoch} times with cmd argument generated by {gen_arg}
    let mut rng = StdRng::seed_from_u64(42);
    let data = TestDate::new(case_count, &mut rng);
    let period = 20;
    let args: Vec<_> = (0..epoch)
        .into_iter()
        .map(|i| {
            let s = if i % period == 0 {
                let tag_num = (i / period) % 3;
                format!("tags +tag{}", tag_num)
            } else {
                let i = rng.gen_range(0..case_count);
                let data = &data.data[i];
                gen_arg(&mut rng, &data.0, &data.1)
            };
            split_args(&s)
        })
        .collect();

    let mut setup_rt = tokio::runtime::Runtime::new().unwrap();
    let mut rt = tokio::runtime::Runtime::new().unwrap();
    b.iter_with_setup(
        || {
            let _ = setup();
            for (name, tag_arr) in data.data.iter() {
                let tag_str = gen_tag_string(tag_arr);
                let args = split_args(&format!(
                    "e --no-template -t {} {} | echo $NAME",
                    tag_str, name
                ));
                setup_rt.block_on(async {
                    let _ = main_with_args(&args).await;
                });
            }
        },
        |_| {
            for arg in args.iter() {
                rt.block_on(async {
                    let _ = main_with_args(arg).await;
                });
            }
        },
    );
}

fn run_criterion(c: &mut Criterion, name: &str, case_count: usize, epoch: usize) {
    if name.contains("fuzzy") {
        c.bench_function(name, |b| {
            run_bench_with(b, case_count, epoch, |rng, name, tag_arr| {
                let name = sample_name(rng, name);
                let filter = gen_tag_filter_string(rng, tag_arr.clone());
                format!("-f +{} {}", filter, name)
            });
        });
    } else if name.contains("exact") {
        c.bench_function(name, |b| {
            run_bench_with(b, case_count, epoch, |rng, name, tag_arr| {
                let filter = gen_tag_filter_string(rng, tag_arr.clone());
                format!("-f +{} ={}", filter, name)
            });
        });
    } else if name.contains("prev") {
        c.bench_function(name, |b| {
            run_bench_with(b, case_count, epoch, |rng, _, _| {
                let filter = gen_tag_string(&gen_tag_arr(rng, -1, 1));
                let prev = rng.gen_range(1..=case_count);
                format!("-f +{} ^{}", filter, prev)
            });
        });
    } else if name.contains("ls") {
        c.bench_function(name, |b| {
            run_bench_with(b, case_count, epoch, |rng, _, tag_arr| {
                let filter = gen_tag_filter_string(rng, tag_arr.clone());
                format!("-f +{} ls", filter)
            });
        });
    } else {
        panic!("看不懂 benchmark 的名字 {}", name);
    }
}

#[criterion]
fn bench_massive_fuzzy(c: &mut Criterion) {
    // run with random tag, with random fuzzy name
    run_criterion(c, "massive_fuzzy", 200, 400);
}
#[criterion]
fn bench_massive_exact(c: &mut Criterion) {
    // run with random tag, with random exact name
    run_criterion(c, "massive_exact", 200, 400);
}
#[criterion]
fn bench_massive_prev(c: &mut Criterion) {
    // run with random tag, with random exact name
    run_criterion(c, "massive_prev", 200, 400);
}
#[criterion]
fn bench_massive_ls(c: &mut Criterion) {
    run_criterion(c, "massive_ls", 200, 400);
}

#[criterion]
fn bench_small_fuzzy(c: &mut Criterion) {
    // run with random tag, with random fuzzy name
    run_criterion(c, "small_fuzzy", 40, 80);
}
#[criterion]
fn bench_small_exact(c: &mut Criterion) {
    // run with random tag, with random exact name
    run_criterion(c, "small_exact", 40, 80);
}
#[criterion]
fn bench_small_prev(c: &mut Criterion) {
    // run with random tag, with random exact name
    run_criterion(c, "small_prev", 40, 80);
}
#[criterion]
fn bench_small_ls(c: &mut Criterion) {
    run_criterion(c, "small_ls", 40, 80);
}
