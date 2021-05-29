#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]

use criterion::{black_box, Criterion};
use criterion_macro::criterion;
use hyper_scripter::fuzzy::*;
use rand::{rngs::StdRng, seq::index::sample, Rng, SeedableRng};
use std::borrow::Cow;

#[criterion]
fn becnch_fuzz(c: &mut Criterion) {
    let _ = env_logger::try_init();

    struct MyStr<'a>(&'a str);
    impl<'a> FuzzKey for MyStr<'a> {
        fn fuzz_key(&self) -> Cow<'a, str> {
            Cow::Borrowed(self.0)
        }
    }

    let mut rng = StdRng::seed_from_u64(42);
    const LONG: usize = 20;
    const SHORT: std::ops::Range<usize> = 5..15;
    const CASE_COUNT: usize = 999;

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

    let mut names = vec![];
    let mut shorts = vec![];
    for _ in 0..CASE_COUNT {
        let name = gen_name(&mut rng);
        let short = sample_name(&mut rng, &name);
        names.push(name);
        shorts.push(short);
    }

    let mut rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("fuzzy", |b| {
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
