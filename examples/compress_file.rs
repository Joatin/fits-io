//! Compressing a file the way `fpack` does, and reporting what it saved.
//!
//! Run with `cargo run --example compress_file -- in.fits out.fits`.

use fits_io::hdu::{HDU, ImageHDU};
use fits_io::image::compression::{Compression, CompressionOptions, Quantize};
use fits_io::{Fits, FitsSlice};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut arguments = std::env::args().skip(1);
    let (input, output) = match (arguments.next(), arguments.next()) {
        (Some(input), Some(output)) => (input, output),
        _ => {
            eprintln!("usage: compress_file <in.fits> <out.fits>");
            std::process::exit(2);
        }
    };

    let before = std::fs::read(&input)?;
    let mut fits = FitsSlice::from_slice(&before)?;

    // Rice coding, in tiles a hundred rows tall. A floating point image is
    // quantised to a quarter of its own noise first, which is what makes one
    // compress at all well — and is lossy, in the low bits that were noise.
    let floating = matches!(
        fits.primary_hdu().header().bitpix(),
        Some(fits_io::header::Bitpix::F32 | fits_io::header::Bitpix::F64)
    );

    let mut options = CompressionOptions::new(Compression::Rice).with_tile_size(&[u32::MAX, 100]);
    if floating {
        options = options.with_quantization(Quantize::NoiseLevel(4.0));
    }

    fits.primary_hdu_mut().compress(&options)?;

    let after = fits.to_vec()?;
    std::fs::write(&output, &after)?;

    println!(
        "{} -> {} bytes ({:.1}% of the original)",
        before.len(),
        after.len(),
        100.0 * after.len() as f64 / before.len() as f64
    );

    Ok(())
}
