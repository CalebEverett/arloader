use arloader::{bundle::DataItem, error::Error, status::Status, Arweave};
use criterion::{
    async_executor::FuturesExecutor,
    {black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput},
};
use rand::Rng;
use rayon::prelude::*;
use std::{fs, path::PathBuf};
use tempdir::TempDir;

fn get_random_bytes(file_size: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut bytes: Vec<u8> = Vec::with_capacity(file_size);
    (0..file_size).for_each(|_| bytes.push(rng.gen()));
    black_box(bytes)
}

fn get_data(file_size: usize, num_files: usize) -> Vec<Vec<u8>> {
    (0..num_files)
        .into_par_iter()
        .map(|_| get_random_bytes(file_size))
        .collect()
}

fn create_data_items(data: Vec<Vec<u8>>) -> Vec<(DataItem, Status)> {
    let arweave = Arweave::from_keypair_path_sync(
        PathBuf::from(
            "tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json",
        ),
        None,
    )
    .unwrap();
    data.into_par_iter()
        .map(|d| {
            let data_item = arweave.create_data_item(d, Vec::new(), false).unwrap();
            (
                arweave.sign_data_item(data_item).unwrap(),
                Status {
                    file_path: Some(PathBuf::new()),
                    ..Default::default()
                },
            )
        })
        .collect()
}

#[allow(dead_code)]
fn files_setup(file_size: usize, num_files: usize) -> Result<TempDir, Error> {
    let mut rng = rand::thread_rng();
    let mut bytes = Vec::with_capacity(file_size);
    (0..file_size).for_each(|_| bytes.push(rng.gen()));

    let temp_dir = tempdir::TempDir::new("test_files")?;

    let _ = (0..num_files).into_par_iter().for_each(|i| {
        fs::write(
            temp_dir.path().join(format!("{}", i)).with_extension("bin"),
            &bytes,
        )
        .unwrap();
    });
    Ok(temp_dir)
}

fn benchmarks(c: &mut Criterion) {
    let arweave = Arweave::from_keypair_path_sync(
        PathBuf::from(
            "tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json",
        ),
        None,
    )
    .unwrap();
    let mut group = c.benchmark_group("benchmarks");
    for (file_size, num_files) in [(15, 500usize), (18, 500), (20, 500), (22, 150), (24, 50)]
        .into_iter()
        .map(|(s, f)| (usize::pow(2, s), f))
    {
        let data = get_random_bytes(file_size);
        group.throughput(Throughput::Bytes(file_size as u64));
        group.bench_with_input(
            BenchmarkId::new("create_data_item", file_size),
            &file_size,
            |b, _| {
                b.iter(|| {
                    let data_item = arweave
                        .create_data_item(data.clone(), Vec::new(), false)
                        .unwrap();
                    let _ = arweave.sign_data_item(data_item);
                })
            },
        );
        let data = black_box(get_data(file_size, num_files));
        group.bench_with_input(
            BenchmarkId::new("create_data_items_in_parallel", file_size * num_files),
            &file_size,
            |b, _| {
                b.iter_batched(
                    || data.clone(),
                    |data| create_data_items(data),
                    criterion::BatchSize::SmallInput,
                )
            },
        );
        let data_items = black_box(create_data_items(data));
        group.bench_with_input(
            BenchmarkId::new("create_bundle_from_data_items", file_size * num_files),
            &file_size,
            |b, _| {
                b.iter_batched(
                    || data_items.clone(),
                    |data_items| arweave.create_bundle_from_data_items(data_items).unwrap(),
                    criterion::BatchSize::SmallInput,
                )
            },
        );
        let (bundle, _) = black_box(arweave.create_bundle_from_data_items(data_items).unwrap());
        group.bench_with_input(
            BenchmarkId::new("create_transaction_from_bundle", file_size * num_files),
            &file_size,
            |b, _| {
                b.iter_batched(
                    || bundle.clone(),
                    |bundle| arweave.process_data(bundle).unwrap(),
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
