use criterion::{criterion_group, criterion_main, Criterion};
use keystroke::infrastructure::font_provider::FontProvider;

fn font_loading_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    c.bench_function("load_system_fonts", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = FontProvider::load_system_fonts_asynchronous().await;
        })
    });
}

criterion_group!(benches, font_loading_benchmark);
criterion_main!(benches);
