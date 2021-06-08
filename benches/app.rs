use criterion::{criterion_group, criterion_main, Criterion};
use salak::*;

fn criterion_benchmark(c: &mut Criterion) {
    let env = Salak::builder()
        .set("hello", "world")
        .register_default_resource::<()>()
        .build()
        .unwrap();

    c.bench_function("res1", |b| b.iter(|| env.init_resource::<()>()));
    c.bench_function("res2", |b| b.iter(|| env.get_resource::<()>()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
