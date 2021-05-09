use criterion::{black_box, criterion_group, criterion_main, Criterion};
use salak::*;
use std::{convert::TryInto, time::Duration};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("env0", |b| b.iter(|| SourceRegistry::new()));
    c.bench_function("env1", |b| b.iter(|| SourceRegistry::default()));
    c.bench_function("env2", |b| b.iter(|| Salak::new().build()));

    let env = Salak::new().set_property("hello", "world").build();

    c.bench_function("hello1", |b| {
        b.iter(|| env.require::<String>(black_box("hello")))
    });

    c.bench_function("hello2", |b| {
        b.iter(|| env.require::<Option<String>>(black_box("hello")))
    });

    c.bench_function("hello3", |b| {
        b.iter(|| env.require::<Option<String>>(black_box("world")))
    });

    c.bench_function("rand", |b| {
        b.iter(|| env.require::<String>(black_box("random.u8")))
    });

    c.bench_function("u8", |b| b.iter(|| Property::Int(1).try_into() == Ok(1u8)));
    c.bench_function("duration", |b| {
        b.iter(|| Property::Str("1s".into()).try_into() == Ok(Duration::from_secs(1)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
