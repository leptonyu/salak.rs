use criterion::{black_box, criterion_group, criterion_main, Criterion};
use salak::*;

fn criterion_benchmark(c: &mut Criterion) {
    let env = Salak::builder().set("hello", "world").build().unwrap();

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
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
