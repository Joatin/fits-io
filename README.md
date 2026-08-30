# FitsIo

A safe, ergonomic, and pure-Rust library for reading and writing FITS (Flexible
Image Transport System) files, inspired by CFITSIO.

This crate offers optional async I/O with Tokio and structured access to FITS
headers, images, and tables — without any C dependencies.

Designed for astronomy, astrophotography, and scientific pipelines where
portability and safety matter.

## Features

* 📦 Pure Rust implementation (no CFITSIO, no C bindings)
* ⚡ Async I/O with Tokio (enabled by default)
* 🧩 Support for Primary HDUs and extensions
* 🖼️ Image HDUs of any dimensionality, cubes and hypercubes included
* 📊 Binary and ASCII tables alike, with optional `serde` support in both directions
* 🗜️ Tile-compressed images, as `fpack` and the archives distribute them
* 🌍 World coordinate helpers: pixels to sky positions and back
* 🧠 Typed access to FITS header keywords
* 🚀 Streaming and memory-efficient reads
* 🛡️ Idiomatic error handling with Result
* 🔁 CFITSIO-inspired API, redesigned for Rust

## Installation

Add the crate to your Cargo.toml:
```toml
[dependencies]
fits-io = "0.1"
```

## Reading a file

```rust,no_run
# #[cfg(feature = "fs")]
# fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::ImageHDU;

let fits = FsFits::open("observation.fits".as_ref())?;

let hdu = fits.primary_hdu();
println!("{} x {}", hdu.images_width(), hdu.images_height());

if let Some(image) = hdu.read_image(0)? {
    let normalised = image.normalized();
    println!("first pixel: {}", normalised.get_pixel(0, 0)[0]);
}
# Ok(())
# }
# fn main() {}
```

## Reading table rows into your own structs

With the `serde` feature, a table's rows deserialize straight into a struct,
matching columns to fields by their TTYPEn names. The same works for an ASCII
table through `read_rows`, `from_ascii_table` and `to_ascii_table`.

```rust,no_run
# #[cfg(all(feature = "fs", feature = "serde"))]
# fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::{BinTableHDU, ExtensionHDU};
use serde::Deserialize;

#[derive(Deserialize)]
struct Source {
    #[serde(rename = "RA")]
    right_ascension: f64,
    #[serde(rename = "DEC")]
    declination: f64,
    // A column with a TNULLn card may leave entries undefined.
    #[serde(rename = "MAG")]
    magnitude: Option<f32>,
}

let fits = FsFits::open("catalogue.fits".as_ref())?;

if let Some(ExtensionHDU::BinTable(hdu)) = fits.extension_hdu(0) {
    let sources: Vec<Source> = hdu.read_rows()?;
    println!("{} sources", sources.len());
}
# Ok(())
# }
# fn main() {}
```

## Writing

Setting data also brings the header into line with it, and saving fills in the
mandatory cards, puts them in the order the standard requires, and writes
CHECKSUM and DATASUM.

```rust,no_run
# #[cfg(feature = "fs")]
# fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::ImageHDU;

let mut fits = FsFits::open("observation.fits".as_ref())?;

fits.primary_hdu_mut()
    .set_raw_images_i16(2, 2, &[&[1, 2, 3, 4]])?;

// `to_vec` returns the bytes; `save` writes them back over the file.
let bytes = fits.to_vec()?;
fits.save()?;
# Ok(())
# }
# fn main() {}
```

### Building a file from nothing

```rust
# #[cfg(feature = "serde")]
# fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
use fits_io::bin_table::to_bin_table;
use fits_io::hdu::{BinTableHDU, ExtensionHDU, ImageHDU};
use fits_io::{Fits, FitsSlice, SliceBinTableHDU};
use serde::Serialize;

#[derive(Serialize)]
struct Star {
    #[serde(rename = "NAME")]
    name: String,
    #[serde(rename = "MAG")]
    magnitude: f64,
}

let mut fits = FitsSlice::new();

fits.primary_hdu_mut()
    .set_raw_images_u8(2, 2, &[&[1, 2, 3, 4]])?;

// Column types and widths are worked out from the rows themselves.
let stars = vec![Star {
    name: "Vega".into(),
    magnitude: 0.03,
}];
let table = SliceBinTableHDU::from_table(&to_bin_table(&stars)?)?;
fits.push_extension(ExtensionHDU::BinTable(table));

let bytes = fits.to_vec()?;
# Ok(())
# }
# fn main() {}
```

## Working without a filesystem

`FitsSlice` reads a file that is already in memory — one arriving over a
network, say, or a build with the `fs` feature turned off. It reads, writes and
streams the same things `FsFits` does, and `from_vec` takes over your buffer
rather than copying it. A gzipped buffer is decompressed transparently.

```rust
use fits_io::{Fits, FitsSlice};
use fits_io::hdu::ImageHDU;

# fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
# let mut bytes = format!("{:<80}{:<80}{:<80}{:<80}{:<80}{:<80}",
#     "SIMPLE  =                    T", "BITPIX  =                    8",
#     "NAXIS   =                    2", "NAXIS1  =                    2",
#     "NAXIS2  =                    2", "END").into_bytes();
# bytes.resize(2880, b' ');
# bytes.extend_from_slice(&[1, 2, 3, 4]);
# bytes.resize(5760, 0);
let fits = FitsSlice::from_slice(&bytes)?;

assert_eq!(fits.primary_hdu().image_count(), 1);
# Ok(())
# }
```

## Feature flags

`default-features = false` gives you header, image and table parsing over
in-memory data through `FitsSlice`, with no filesystem, async or threading
support.

| Feature | Default | Effect                                                    |
|---------|---------|-----------------------------------------------------------|
| `fs`    | ✅      | Read and write FITS files on the filesystem via `FsFits`  |
| `gzip`  | ✅      | Transparently decompress gzipped files and buffers        |
| `tokio` | ✅      | Async open and streaming reads                            |
| `rayon` | ✅      | Parallel table row decoding, worth about 4x on a big table |
| `serde` |         | Convert table rows to and from your own structs           |

## Benchmarks

`cargo bench --features fs,serde,rayon` times opening a file, reading a table
and decoding its rows, against the Gaia fixture under `tests/`. It reports the
fastest of several runs and skips when the fixture is absent.

## Documentation

Every public item is documented, and `#![deny(missing_docs)]` keeps it that way.
The API docs are on [docs.rs](https://docs.rs/fits-io), built with every feature
enabled so the `serde`, `tokio` and `gzip` parts are visible.

## Design Goals

* **Safety** — eliminate undefined behavior and unsafe FFI
* **Portability** — run anywhere Rust runs
* **Ergonomics** — minimal boilerplate
* **Performance** — streaming-friendly, low overhead
* **Familiarity** — CFITSIO-inspired, Rust-native

## Supported FITS Features

| Feature                       | Status | Notes                                                    |
|-------------------------------|--------|----------------------------------------------------------|
| Primary HDU                   | ✅      |                                                          |
| Extension HDUs                | ✅      |                                                          |
| Image HDU                     | ✅      | Any number of axes; planes beyond the second are indexed |
| Binary tables                 | ✅      | `serde` converts rows to and from your structs           |
| ASCII tables                  | ✅      | Read, written, streamed and `serde`-mapped like binary ones |
| Variable-length array columns | ✅      | TFORMn `P` and `Q`, read and written through the heap    |
| Complex columns               | ✅      | TFORMn `C` and `M`                                       |
| Column scaling                | ✅      | TSCALn, TZEROn and TNULLn applied both ways              |
| Unsigned columns              | ✅      | The TZEROn convention, at all four integer widths        |
| Multidimensional columns      | ✅      | TDIMn read and written, nesting as deep as it says       |
| Undefined image pixels        | ✅      | BLANK reads as NaN rather than as black                  |
| Header read                   | ✅      |                                                          |
| Header write                  | ✅      | Mandatory cards filled in and ordered as the standard asks |
| Image write                   | ✅      | `set_raw_images_*`, then `save` or `to_vec`              |
| Table write                   | ✅      | `set_table` / `set_rows`, for both table kinds           |
| Building files                 | ✅      | `push_extension` and `remove_extension`                  |
| Gzip decompression            | ✅      | `.fits.gz` files and gzipped buffers alike               |
| Streaming image reads         | ✅      | `stream_normalised_image`, via the `tokio` feature       |
| Streaming table rows          | ✅      | `stream_table_rows`, via the `tokio` feature             |
| WCS helpers                   | ✅      | `CDi_j`, `PCi_j` and `CDELTn`/`CROTAn`; linear and `TAN` |
| CHECKSUM and DATASUM          | ✅      | Written on save; `checksum::verify` checks an HDU        |
| Random groups                 | ✅      | `group_count` and `read_group`, with PSCALn and PZEROn   |
| Compressed image extensions   | ✅      | Tiled `RICE_1`, `GZIP_1`, `GZIP_2` and `NOCOMPRESS`      |

## License

Licensed under either of:

* Apache License, Version 2.0
* MIT License

at your option.

## Contributing

Issues, discussions, and pull requests are welcome.
Please open an issue for large changes or new features.

## Acknowledgements

Inspired by CFITSIO and the FITS standard maintained by NASA/HEASARC.

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
