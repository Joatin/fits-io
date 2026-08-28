mod common;

use common::{fits_file, fixture, write_temp_fits};
use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::ImageHDU;
use futures::StreamExt;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

const CAMERA_FRAME: &str = "Light_19 Aurigae_600.0s_Bin1_ISO800_20251117-190134_6.0C_0001.fit";

/// A 4x3 unsigned-16-bit image, the encoding astro cameras produce: BITPIX 16
/// with BZERO 32768. Raw i16 -32768 is physical 0, raw 32767 is physical 65535.
fn unsigned_16_bit_image() -> (Vec<u8>, Vec<i16>) {
    let raw: Vec<i16> = vec![
        i16::MIN,
        -16384,
        0,
        16384, //
        i16::MAX,
        0,
        i16::MIN,
        i16::MAX, //
        -1,
        1,
        -32767,
        32766,
    ];

    let mut data = Vec::new();
    for value in &raw {
        data.extend_from_slice(&value.to_be_bytes());
    }

    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "16"),
            ("NAXIS", "2"),
            ("NAXIS1", "4"),
            ("NAXIS2", "3"),
            ("BZERO", "32768"),
            ("BSCALE", "1"),
        ],
        &data,
    );

    (file, raw)
}

#[tokio::test]
async fn streams_every_pixel_with_its_normalised_value() -> TestResult {
    let (file, raw) = unsigned_16_bit_image();
    let path = write_temp_fits("unsigned16.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let stream = fits
        .primary_hdu()
        .stream_normalised_image(0)?
        .expect("image 0 exists");

    let streamed: Vec<(u32, u32, f64)> = stream.collect().await;

    assert_eq!(streamed.len(), raw.len(), "one triple per pixel");

    for (index, (x, y, value)) in streamed.iter().enumerate() {
        // Pixels arrive left to right, top to bottom.
        assert_eq!((*x, *y), (index as u32 % 4, index as u32 / 4), "at {index}");

        // physical = BZERO + BSCALE * raw, normalised over the full 16-bit range.
        let expected = (raw[index] as f64 + 32768.0) / 65535.0;
        assert!(
            (value - expected).abs() < 1e-9,
            "pixel {index}: streamed {value}, expected {expected}"
        );
    }

    // The endpoints of the range really do reach 0.0 and 1.0.
    assert_eq!(streamed[0].2, 0.0);
    assert_eq!(streamed[4].2, 1.0);

    Ok(())
}

#[tokio::test]
async fn streamed_values_are_not_all_zero() -> TestResult {
    // Guards the specific regression where the decoded byte was discarded and
    // every pixel was reported as 0.0.
    let (file, _) = unsigned_16_bit_image();
    let path = write_temp_fits("nonzero.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let streamed: Vec<(u32, u32, f64)> = fits
        .primary_hdu()
        .stream_normalised_image(0)?
        .expect("image 0 exists")
        .collect()
        .await;

    let distinct = streamed
        .iter()
        .map(|(_, _, value)| value.to_bits())
        .collect::<std::collections::HashSet<_>>();

    assert!(
        distinct.len() > 1,
        "expected varying pixel values, got {distinct:?}"
    );
    assert!(streamed.iter().any(|(_, _, value)| *value > 0.0));

    Ok(())
}

#[tokio::test]
async fn streams_8_bit_images_one_pixel_per_byte() -> TestResult {
    let data: Vec<u8> = vec![0, 51, 102, 153, 204, 255];
    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "3"),
            ("NAXIS2", "2"),
        ],
        &data,
    );
    let path = write_temp_fits("unsigned8.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let streamed: Vec<(u32, u32, f64)> = fits
        .primary_hdu()
        .stream_normalised_image(0)?
        .expect("image 0 exists")
        .collect()
        .await;

    assert_eq!(streamed.len(), 6);
    assert_eq!(streamed[0], (0, 0, 0.0));
    assert_eq!(streamed[5], (2, 1, 1.0));

    // No BZERO or BSCALE cards: the standard defaults leave the data unshifted.
    for (index, (_, _, value)) in streamed.iter().enumerate() {
        let expected = data[index] as f64 / 255.0;
        assert!((value - expected).abs() < 1e-9, "pixel {index}");
    }

    Ok(())
}

#[tokio::test]
async fn an_hdu_without_image_data_streams_nothing() -> TestResult {
    let file = fits_file(&[("SIMPLE", "T"), ("BITPIX", "8"), ("NAXIS", "0")], &[]);
    let path = write_temp_fits("headeronly.fits", &file)?;

    let fits = FsFits::open(&path)?;
    assert!(fits.primary_hdu().stream_normalised_image(0)?.is_none());

    Ok(())
}

#[tokio::test]
async fn floating_point_images_need_a_known_range() -> TestResult {
    // A float image has no representable full-scale range, so without DATAMIN and
    // DATAMAX there is no honest single-pass normalisation.
    let data: Vec<u8> = 1.5_f32
        .to_be_bytes()
        .into_iter()
        .chain(2.5_f32.to_be_bytes())
        .collect();
    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "-32"),
            ("NAXIS", "2"),
            ("NAXIS1", "2"),
            ("NAXIS2", "1"),
        ],
        &data,
    );
    let path = write_temp_fits("float.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let Err(error) = fits.primary_hdu().stream_normalised_image(0) else {
        panic!("a float image without DATAMIN/DATAMAX cannot be normalised");
    };

    assert!(
        error.to_string().contains("DATAMIN"),
        "error should name the missing cards, got: {error}"
    );

    Ok(())
}

#[tokio::test]
async fn floating_point_images_stream_when_datamin_and_datamax_are_present() -> TestResult {
    let data: Vec<u8> = 1.5_f32
        .to_be_bytes()
        .into_iter()
        .chain(2.5_f32.to_be_bytes())
        .chain(2.0_f32.to_be_bytes())
        .chain(1.0_f32.to_be_bytes())
        .collect();
    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "-32"),
            ("NAXIS", "2"),
            ("NAXIS1", "2"),
            ("NAXIS2", "2"),
            ("DATAMIN", "1.0"),
            ("DATAMAX", "2.5"),
        ],
        &data,
    );
    let path = write_temp_fits("float-ranged.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let streamed: Vec<(u32, u32, f64)> = fits
        .primary_hdu()
        .stream_normalised_image(0)?
        .expect("image 0 exists")
        .collect()
        .await;

    let values: Vec<f64> = streamed.iter().map(|(_, _, value)| *value).collect();
    let expected = [(1.5 - 1.0) / 1.5, (2.5 - 1.0) / 1.5, (2.0 - 1.0) / 1.5, 0.0];

    for (index, expected) in expected.iter().enumerate() {
        assert!(
            (values[index] - expected).abs() < 1e-6,
            "pixel {index}: streamed {}, expected {expected}",
            values[index]
        );
    }

    Ok(())
}

#[tokio::test]
async fn streams_a_real_camera_frame() -> TestResult {
    let Some(path) = fixture(CAMERA_FRAME) else {
        return Ok(());
    };

    let fits = FsFits::open_async(&path).await?;
    let primary_hdu = fits.primary_hdu();

    let width = primary_hdu.images_width();
    let height = primary_hdu.images_height();

    let stream = primary_hdu
        .stream_normalised_image(0)?
        .expect("image 0 exists");

    let (count, sum, out_of_range) = stream
        .fold(
            (0_u64, 0.0_f64, 0_u64),
            |(count, sum, bad), (_, _, value)| async move {
                let bad = bad + u64::from(!(0.0..=1.0).contains(&value));
                (count + 1, sum + value, bad)
            },
        )
        .await;

    assert_eq!(count, width as u64 * height as u64, "one triple per pixel");
    assert_eq!(
        out_of_range, 0,
        "every value must be normalised to 0.0..=1.0"
    );
    assert!(sum > 0.0, "a real light frame is not uniformly black");

    Ok(())
}

#[tokio::test]
async fn streaming_and_reading_agree_on_normalised_values() -> TestResult {
    // The streaming and in-memory paths share one Normalizer, so they must not
    // drift apart again.
    let (file, _) = unsigned_16_bit_image();
    let path = write_temp_fits("agreement.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let hdu = fits.primary_hdu();

    let streamed: Vec<(u32, u32, f64)> = hdu
        .stream_normalised_image(0)?
        .expect("image 0 exists")
        .collect()
        .await;

    let normalized = hdu.read_image(0)?.expect("image 0 exists").normalized();

    for (x, y, value) in streamed {
        let expected = normalized.get_pixel(x, y)[0];
        assert!(
            (value - expected).abs() < 1e-12,
            "({x}, {y}): streamed {value}, read_image gave {expected}"
        );
    }

    Ok(())
}

#[tokio::test]
async fn streaming_and_reading_agree_on_a_real_camera_frame() -> TestResult {
    let Some(path) = fixture(CAMERA_FRAME) else {
        return Ok(());
    };

    let fits = FsFits::open_async(&path).await?;
    let hdu = fits.primary_hdu();

    let normalized = hdu.read_image(0)?.expect("image 0 exists").normalized();

    // Checking every one of 32M pixels is needlessly slow; a stride samples the
    // whole frame just as well.
    let mismatches = hdu
        .stream_normalised_image(0)?
        .expect("image 0 exists")
        .enumerate()
        .filter(|(index, _)| futures::future::ready(index % 9973 == 0))
        .filter(|(_, (x, y, value))| {
            futures::future::ready((value - normalized.get_pixel(*x, *y)[0]).abs() > 1e-12)
        })
        .count()
        .await;

    assert_eq!(mismatches, 0, "streamed values must match read_image");

    Ok(())
}
