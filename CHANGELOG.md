# Changelog

All notable changes to this crate are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-08-31

### Added

#### Header editing

* `Header::set_card` and `Header::remove_card` set and remove a card by keyword,
  for the keywords this crate has no accessor of its own for. A keyword of up to
  eight conventional characters is written as an ordinary card and a longer one
  as a `HIERARCH` card; a string too long for one card is written across
  `CONTINUE` cards. Setting a keyword the crate does know leaves its typed
  accessor returning the new value.
* `Header::card`, `Header::contains_card` and `Header::card_keys` read back what
  a header holds.
* `Header::add_comment`, `Header::add_history`, `Header::comments` and
  `Header::history` handle the two repeatable keywords, splitting text too long
  for one card across as many as it needs.
* `Value` is exported, along with `From` conversions from the Rust types a card
  can hold and `Value::with_comment`.

#### Writing compressed images

* `ImageHDU::compress` and `ImageHDU::decompress` store an HDU's image
  tile-compressed and back again, the way `fpack` does. Every algorithm this
  crate reads it now also writes: `RICE_1`, `HCOMPRESS_1`, `PLIO_1`, `GZIP_1`,
  `GZIP_2` and `NOCOMPRESS`. The tile shape, the coding block size, the
  HCOMPRESS scale factor and the dithering are all settable through
  `CompressionOptions`.
* A floating point image can be quantised first, in steps given outright or as a
  fraction of the tile's own estimated noise, with `SUBTRACTIVE_DITHER_1` or
  `SUBTRACTIVE_DITHER_2` dithering.
* A compressed image is written as the binary table extension the convention
  stores it in. An image compressed in the primary HDU moves into the first
  extension when the file is written, since the primary HDU cannot hold a table.

#### Reading compressed images

* Compressed images of any number of axes, rather than two.
* `ZQUANTIZ` dithering is undone on the way in. A dithered floating point image
  written by `fpack` previously came back wrong by up to half a quantisation
  step, in a pattern rather than at random.
* `ZBLANK` marks a pixel the image does not define, which comes back as `NaN`.
* HCOMPRESS `SMOOTH` is honoured rather than refused, matching the reference
  implementation value for value.

#### World coordinates

* Nine more projections: `SIN`, `STG`, `ARC`, `ZEA`, `CAR`, `MER`, `CEA`, `AIT`
  and `MOL`, alongside `TAN`.
* `SIP` and `TPV` distortions, applied and inverted — between them, what most
  plate solvers write.
* `LONPOLE` and `LATPOLE` orient the projection, through the standard's own
  rotation rather than a formula that assumed the usual orientation.
* `Wcs::pixel_to_world_axis` and `Wcs::world_to_pixel_axis` read a cube's third
  axis and beyond, linear or `-LOG`, with `Wcs::axis_count`, `Wcs::axis_type`
  and `Wcs::axis_unit` describing them.
* `Wcs::is_celestial` and `Wcs::celestial_pole`.

#### Tables

* A row may be a map of column names to values, which is what a `HashMap` row
  is and what `#[serde(flatten)]` produces — so a row made of nested structs now
  writes as columns of its own.
* Rows whose columns arrive in different orders, as a `HashMap`'s do, are lined
  up rather than reported as rows that disagree.

### Fixed

* Rice decoding read a block stored verbatim at the wrong width, and its bit
  reader could not hold a whole 32-bit value. A tile with a high-entropy block
  in it — noise, or any data whose differences are as large as its values —
  decoded to the wrong numbers.
* A compressed cube handed back its whole array for every plane rather than the
  plane that was asked for.

### Changed

* `Wcs` is no longer `Copy`, since it now carries a description of every axis.
* `Projection` has a variant per projection rather than two.
* A celestial axis whose `CUNITn` is not degrees is refused rather than read as
  though it were.
