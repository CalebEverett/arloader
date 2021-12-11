use arloader::{commands::*, error::Error, Arweave};
use glob::glob;
use image::Rgb;
use imageproc::drawing::draw_text_mut;
use rand::Rng;
use rusttype::{Font, Scale};
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> CommandResult {
    let arweave = Arweave::default();
    let sol_keypair_path = env::var("SOL_KEYPAIR_PATH");

    if sol_keypair_path.is_err() {
        println!("Example requires SOL_KEYPAIR_PATH environment variable to be set.");
        return Ok(());
    };
    let sol_keypair_path = sol_keypair_path.unwrap();

    // Generate images and metadata.
    let temp_dir = files_setup()?;

    // Create log dirs for both images and metadata.
    let images_log_dir = temp_dir.join("status/images/");
    fs::create_dir_all(images_log_dir.clone()).unwrap();
    let metadata_log_dir = temp_dir.join("status/metadata/");
    fs::create_dir_all(metadata_log_dir.clone()).unwrap();
    let images_glob_str = format!("{}/*.png", temp_dir.display().to_string());
    let images_log_dir_str = images_log_dir.display().to_string();
    let metadata_glob_str = format!("{}/*.json", temp_dir.display().to_string());
    let metadata_log_dir_str = metadata_log_dir.display().to_string();

    // Upload images
    println!("\n\nUploading images...\n");
    command_upload_bundles_with_sol(
        &arweave,
        &images_glob_str,
        Some(images_log_dir_str.clone()),
        None,
        1_000_000,
        10.0,
        None,
        5,
        &sol_keypair_path,
    )
    .await?;

    println!("\n\nUploading manifest for images...\n");
    command_upload_manifest(
        &arweave,
        &images_log_dir_str,
        10.0,
        Some(sol_keypair_path.clone()),
    )
    .await?;

    let manifest_str = glob(&format!("{}/manifest*.json", images_log_dir_str))
        .unwrap()
        .filter_map(Result::ok)
        .nth(0)
        .unwrap()
        .display()
        .to_string();

    // Update metadata with links to uploaded images. The glob should match the images in order to match
    // the links that were written to the manifest file. We pass in `true` for `link_file` to use the
    // link with the file name relative to the manifest transaction id to update the metadata `image` key.
    println!("\n\nUpdating metadata with links from manifest...\n");
    command_update_metadata(&arweave, &images_glob_str, &manifest_str, true).await?;

    println!("\n\nUploading updated metadata files...\n");
    command_upload_bundles_with_sol(
        &arweave,
        &metadata_glob_str,
        Some(metadata_log_dir_str.clone()),
        None,
        1_000_000,
        10.0,
        None,
        5,
        &sol_keypair_path,
    )
    .await?;

    println!("\n\nUploading manifest for metadata...\n");
    command_upload_manifest(
        &arweave,
        &metadata_log_dir_str,
        10.0,
        Some(sol_keypair_path),
    )
    .await?;
    let manifest_path = glob(&format!("{}/manifest*.json", metadata_log_dir_str))
        .unwrap()
        .filter_map(Result::ok)
        .nth(0)
        .unwrap();

    let manifest: serde_json::Value =
        serde_json::from_str(&tokio::fs::read_to_string(manifest_path).await.unwrap()).unwrap();

    // You should be able to click on these links and see the updated metadata files, which, in turn will have
    // links to the uploaded images. It may take a few minutes before the manifest transaction is mined.
    println!("\n\nHere are the uploaded links, ready to be used to create tokens!\n");
    println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
    Ok(())
}

fn files_setup() -> Result<PathBuf, Error> {
    let temp_dir = PathBuf::from("target/examples/upload_nfts/");
    fs::create_dir_all(temp_dir.clone()).unwrap();
    let mut rng = rand::thread_rng();

    let _ = (0..5).into_iter().for_each(|i| {
        let rd: f32 = rng.gen_range(0.1..0.4);
        let bd: f32 = rng.gen_range(0.1..0.4);
        let c0: f32 = rng.gen_range(-0.5..-0.3);
        let c1: f32 = rng.gen_range(0.5..0.7);
        generate_image(temp_dir.join(format!("{}.png", i)), i, rd, bd, c0, c1);

        fs::write(
            temp_dir.join(format!("{}.json", i)),
            serde_json::to_string(&json!({
                "name": format!("Arloader NFT #{}", i),
                "description": "Super dope, one of a kind NFT",
                "collection": {"name": "Arloader NFT", "family": "We AR"},
                "properties": {"category": "image"},
            }))
            .unwrap(),
        )
        .unwrap();
    });
    Ok(temp_dir)
}

fn generate_image(file_path: PathBuf, i: i32, rd: f32, bd: f32, c0: f32, c1: f32) {
    let imgx = 400;
    let imgy = 400;

    let scalex = 1.5 / imgx as f32;
    let scaley = 1.5 / imgy as f32;

    let mut imgbuf = image::ImageBuffer::new(imgx, imgy);

    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let r = (rd * x as f32) as u8;
        let b = (bd * y as f32) as u8;
        *pixel = image::Rgb([r, 0, b]);
    }
    for x in 0..imgx {
        for y in 0..imgy {
            let cx = y as f32 * scalex - 0.75;
            let cy = x as f32 * scaley - 0.75;

            let c = num_complex::Complex::new(c0, c1);
            let mut z = num_complex::Complex::new(cx, cy);

            let mut i = 0;
            while i < 255 && z.norm() <= 2.0 {
                z = z * z + c;
                i += 1;
            }

            let pixel = imgbuf.get_pixel_mut(x, y);
            let data = (*pixel as image::Rgb<u8>).0;
            *pixel = image::Rgb([data[0], i as u8, data[2]]);
        }
    }

    let font = Vec::from(include_bytes!("../tests/fixtures/DejaVuSans.ttf") as &[u8]);
    let font = Font::try_from_vec(font).unwrap();

    let height = 24.0;
    let scale = Scale {
        x: height,
        y: height,
    };

    let text = format!("Arloader NFT #{}", i);
    draw_text_mut(
        &mut imgbuf,
        Rgb([255u8, 255u8, 255u8]),
        180,
        200,
        scale,
        &font,
        &text,
    );

    imgbuf.save(file_path).unwrap();
}
