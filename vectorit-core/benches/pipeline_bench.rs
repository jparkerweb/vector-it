use criterion::{black_box, criterion_group, criterion_main, Criterion};
use vectorit_core::pipeline;
use vectorit_core::types::{RawImage, VectorizationConfig};

fn generate_test_image(width: u32, height: u32) -> RawImage {
    let total = (width * height) as usize;
    let mut pixels = Vec::with_capacity(total);
    for y in 0..height {
        for x in 0..width {
            // Create a simple gradient pattern with distinct regions
            let r = ((x * 255) / width) as u8;
            let g = ((y * 255) / height) as u8;
            let b = (((x + y) * 128) / (width + height)) as u8;
            pixels.push([r, g, b, 255]);
        }
    }
    RawImage {
        width,
        height,
        pixels,
        has_alpha: false,
    }
}

fn bench_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline");
    group.sample_size(10);

    let config = VectorizationConfig {
        color_count: 8,
        auto_resize: false,
        ..VectorizationConfig::default()
    };

    // 256×256 = 65K pixels
    let img_256 = generate_test_image(256, 256);
    group.bench_function("256x256", |b| {
        b.iter(|| {
            pipeline::vectorize(black_box(img_256.clone()), black_box(&config), None).unwrap();
        })
    });

    // 512×512 = 262K pixels
    let img_512 = generate_test_image(512, 512);
    group.bench_function("512x512", |b| {
        b.iter(|| {
            pipeline::vectorize(black_box(img_512.clone()), black_box(&config), None).unwrap();
        })
    });

    // 1024×1024 = 1M pixels
    let img_1024 = generate_test_image(1024, 1024);
    group.bench_function("1024x1024", |b| {
        b.iter(|| {
            pipeline::vectorize(black_box(img_1024.clone()), black_box(&config), None).unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, bench_pipeline);
criterion_main!(benches);
