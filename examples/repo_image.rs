use arloader::{commands::*, error::Error};
use image::Rgb;
use imageproc::drawing::draw_text;
use rand::Rng;
use rayon::prelude::*;
use rusttype::{Font, Scale};
use std::fs;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> CommandResult {
    // Generate images and metadata.
    println!("\n\nCreating images...\n");
    files_setup(10, 1280, 640, 44, "arloader", 156.0)?;

    Ok(())
}

fn files_setup(
    num_nfts: i32,
    width: u32,
    height: u32,
    iters: usize,
    text: &str,
    font_size: f32,
) -> Result<PathBuf, Error> {
    let temp_dir = PathBuf::from("target/examples/repo_image");
    fs::create_dir_all(&temp_dir)?;

    let font = Vec::from(include_bytes!("../tests/fixtures/OpenSans-Semibold.ttf") as &[u8]);
    let font = Font::try_from_vec(font).unwrap();
    let mut rng = rand::thread_rng();

    let _ = (0..num_nfts).into_iter().for_each(|i| {
        let cx: f64 = rng.gen_range(-0.9..-0.3);
        let cy: f64 = rng.gen_range(0.5..0.6);

        generate_image(
            temp_dir.join(format!("{}.png", i)),
            width,
            height,
            cx,
            cy,
            iters,
            text,
            &font,
            font_size,
        );
    });
    Ok(temp_dir)
}

fn generate_image(
    file_path: PathBuf,
    width: u32,
    height: u32,
    cx: f64,
    cy: f64,
    iters: usize,
    text: &str,
    font: &Font,
    font_size: f32,
) {
    let imgbuf = generate_julia_fractal(width, height, cx, cy, iters);
    let imgbuf = add_text(text, font, width / 2 - 30, height / 2, font_size, imgbuf);
    imgbuf.save(file_path).unwrap();
}

fn generate_julia_fractal(
    width: u32,
    height: u32,
    cx: f64,
    cy: f64,
    iters: usize,
) -> image::RgbImage {
    let mut image = image::ImageBuffer::new(width, height);
    let c = num_complex::Complex64::new(cx as f64, cy);

    image.par_chunks_mut(3).enumerate().for_each(|(i, p)| {
        let (x, y) = index_to_coordinates(i as u32, width);
        let inner_height = height as f64;
        let inner_width = width as f64;
        let inner_y = y as f64;
        let inner_x = x as f64;

        let zx = 2.0 * (inner_x - 0.7 * inner_width) / (inner_width * 1.4);
        let zy = 1.3 * (inner_y - 0.3 * inner_height) / (inner_height * 1.4);

        let mut i = iters;

        let mut z = num_complex::Complex64::new(zx, zy);
        while (z + z).re <= 4.0 && i > 1 {
            z = z * z + c;
            i -= 1;
        }

        let r = (i << 4) as u8;
        let g = (i << 6) as u8;
        let b = (i * 3) as u8;
        let pixel = into_rgb(r, g, b);
        p.copy_from_slice(&pixel);
    });

    image
}

fn index_to_coordinates(idx: u32, length: u32) -> (u32, u32) {
    let x = idx % length;
    let y = idx / length;
    (x, y)
}

fn into_rgb(r: u8, g: u8, b: u8) -> [u8; 3] {
    [r, g, b]
}

fn add_text(
    text: &str,
    font: &Font,
    x: u32,
    y: u32,
    height: f32,
    mut imgbuf: image::ImageBuffer<Rgb<u8>, Vec<u8>>,
) -> image::RgbImage {
    let scale = Scale {
        x: height,
        y: height,
    };

    let imgbuf = draw_text(
        &mut imgbuf,
        Rgb([255u8, 255u8, 255u8]),
        x,
        y,
        scale,
        &font,
        &text,
    );

    imgbuf
}
