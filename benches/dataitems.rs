use arloader::{error::Error, Arweave};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rayon::prelude::*;
use std::{fs, path::PathBuf};
use tempdir::TempDir;

fn files_setup() -> Result<(TempDir), Error> {
    let arweave = Arweave::from_keypair_path_sync(
        PathBuf::from(
            "tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json",
        ),
        None,
    );

    let file_path = PathBuf::from("tests/fixtures/1mb.bin");
    let temp_dir = tempdir::TempDir::new("./tests/")?;

    let _ = (0..100usize).into_par_iter().for_each(|i| {
        fs::copy(
            file_path.clone(),
            temp_dir.path().join(format!("{}", i)).with_extension("bin"),
        )
        .unwrap();
    });
    Ok(temp_dir)
}

async fn create_data_items() -> Result<(), Error> {
    // let glob_str = format!("{}/*.bin", temp_dir.0.display().to_string());
    // let paths_iter = glob(&glob_str)?.filter_map(Result::ok).collect();
    // let pre_data_items = arweave
    //     .create_data_items_from_file_paths(paths_iter, Vec::new())
    //     .await?;
    // let duration = start.elapsed() - duration;
    // println!(
    //     "Time elapsed to create data items from file paths: {} ms",
    //     duration.as_millis()
    // );

    // let start = Instant::now();
    // let (bundle, _) = arweave.create_bundle_from_data_items(pre_data_items.clone())?;
    // let duration = start.elapsed();
    // println!("Time elapsed to create bundle: {} ms", duration.as_millis());

    // let start = Instant::now();
    // let _ = arweave.create_transaction(bundle.clone(), None, None, (0, 0), true);
    // let duration = start.elapsed();
    // println!(
    //     "Time elapsed to create transaction: {} ms",
    //     duration.as_millis()
    // );

    // let start = Instant::now();
    // let post_data_items = arweave.deserialize_bundle(bundle)?;
    // let duration = start.elapsed();
    // println!("Time elapsed to deserialize: {} ms", duration.as_millis());
    // assert_eq!(post_data_items.len(), 100);

    Ok(())
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
