use criterion::{criterion_group, criterion_main, Criterion};
use salak::*;

fn criterion_benchmark(c: &mut Criterion) {
    let env = Salak::builder()
        .set("hello", "world")
        .register_default_resource::<()>()
        .unwrap()
        .build()
        .unwrap();

    c.bench_function("res1", |b| b.iter(|| env.init_resource::<()>()));
    c.bench_function("res2", |b| b.iter(|| env.get_resource::<()>()));
    let mut builder = Salak::builder().set("hello", "world");

    builder = builder
        .register_default_resource::<()>()
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-0"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-1"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-2"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-3"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-4"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-5"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-6"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-7"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-8"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-10"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-11"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-12"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-13"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-14"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-15"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-16"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-17"))
        .unwrap()
        .register_resource::<()>(ResourceBuilder::new("res-18"))
        .unwrap();
    let env = builder.build().unwrap();
    c.bench_function("res3", |b| b.iter(|| env.get_resource::<()>()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
