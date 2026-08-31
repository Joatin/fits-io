//! Turning pixels into sky positions, and back.
//!
//! Run with `cargo run --example sky_coordinates -- image.fits`.

use fits_io::hdu::{HDU, ImageHDU};
use fits_io::wcs::Wcs;
use fits_io::{Fits, FitsSlice};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let path = std::env::args().nth(1).expect("a FITS file to read");
    let fits = FitsSlice::from_slice(&std::fs::read(&path)?)?;

    let hdu = fits.primary_hdu();

    // A compressed image keeps the image's own cards under `Z` keywords, and
    // this is the header the image would have had.
    let header = if hdu.is_compressed() {
        hdu.header().uncompressed()
    } else {
        hdu.header().clone()
    };

    let wcs = Wcs::from_header(&header)?;
    let (width, height) = (hdu.images_width(), hdu.images_height());

    println!("projection: {}", wcs.projection().code());
    println!("{width} x {height} pixels");

    for (name, pixel) in [
        ("first pixel", (1.0, 1.0)),
        ("centre", (width as f64 / 2.0, height as f64 / 2.0)),
        ("last pixel", (width as f64, height as f64)),
    ] {
        let (longitude, latitude) = wcs.pixel_to_world(pixel);
        println!("{name:>12}: {longitude:10.5} {latitude:+10.5}");
    }

    // And back again, to the pixel a coordinate falls on.
    let centre = wcs.pixel_to_world((width as f64 / 2.0, height as f64 / 2.0));
    match wcs.world_to_pixel_indexed(centre, width, height) {
        Some((x, y)) => println!("the centre coordinate is at array index ({x}, {y})"),
        None => println!("the centre coordinate falls outside the image"),
    }

    // Anything beyond the first two axes reads on its own.
    for axis in 2..wcs.axis_count() {
        let unit = wcs.axis_unit(axis).unwrap_or("");
        if let Some(value) = wcs.pixel_to_world_axis(axis, 1.0) {
            println!(
                "axis {} ({}): first plane at {value} {unit}",
                axis + 1,
                wcs.axis_type(axis).unwrap_or("unnamed"),
            );
        }
    }

    Ok(())
}
