//! Building a FITS file from nothing: an image, a header describing it, a table
//! of sources beside it, and the whole thing written out.
//!
//! Run with `cargo run --example write_file --features serde -- out.fits`.

use fits_io::bin_table::to_bin_table;
use fits_io::hdu::{ExtensionHDU, HDU, ImageHDU};
use fits_io::header::Value;
use fits_io::{Fits, FitsSlice, SliceBinTableHDU};
use serde::Serialize;
use std::error::Error;

#[derive(Serialize)]
struct Source {
    #[serde(rename = "NAME")]
    name: String,
    #[serde(rename = "RA")]
    right_ascension: f64,
    #[serde(rename = "DEC")]
    declination: f64,
    #[serde(rename = "MAG")]
    magnitude: f32,
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let path = std::env::args().nth(1).unwrap_or_else(|| "out.fits".into());

    let mut fits = FitsSlice::new();

    // A small image: a gradient with a star dropped into the middle of it.
    let (width, height) = (64_u32, 64_u32);
    let pixels: Vec<i16> = (0..(width * height))
        .map(|index| {
            let (x, y) = (index % width, index / width);
            let background = 100 + (x as i16) / 4 + (y as i16) / 4;

            if x.abs_diff(32) < 2 && y.abs_diff(32) < 2 {
                background + 4000
            } else {
                background
            }
        })
        .collect();

    fits.primary_hdu_mut()
        .set_raw_images_i16(width, height, &[&pixels])?;

    // The header: what this is, and where on the sky it points.
    let header = fits.primary_hdu_mut().header_mut();

    header.set_card("OBJECT", "NGC 7000")?;
    header.set_card("TELESCOP", "Example 200mm")?;
    header.set_card("EXPTIME", Value::from(300.0).with_comment("seconds"))?;
    header.set_card("BUNIT", "adu")?;

    header.set_card("CTYPE1", "RA---TAN")?;
    header.set_card("CTYPE2", "DEC--TAN")?;
    header.set_card("CRPIX1", 32.5)?;
    header.set_card("CRPIX2", 32.5)?;
    header.set_card("CRVAL1", 314.75)?;
    header.set_card("CRVAL2", 44.37)?;
    header.set_card("CDELT1", -0.000_5)?;
    header.set_card("CDELT2", 0.000_5)?;

    header.add_history("written by the fits-io write_file example");

    // A catalogue of what is in the frame, as an extension.
    let sources = vec![
        Source {
            name: "one".into(),
            right_ascension: 314.751,
            declination: 44.370,
            magnitude: 8.2,
        },
        Source {
            name: "two".into(),
            right_ascension: 314.760,
            declination: 44.381,
            magnitude: 11.6,
        },
    ];

    let table = SliceBinTableHDU::from_table(&to_bin_table(&sources)?)?;
    fits.push_extension(ExtensionHDU::BinTable(table));

    std::fs::write(&path, fits.to_vec()?)?;
    println!("wrote {path}");

    Ok(())
}
