#![cfg_attr(nightly, feature(doc_cfg))]
// `header::header::Header` and friends: the inner module is an implementation
// detail that the parent re-exports.
#![allow(clippy::module_inception)]
#![doc = include_str!("../README.md")]

pub mod ansi_table;
pub mod bin_table;
mod error;
mod fits;
#[cfg(feature = "fs")]
pub mod fs;
pub mod hdu;
pub mod header;
pub mod image;
mod result;
mod util;

// An in-memory FITS reader, built on a borrowed buffer rather than a file.
//
// Not finished: `FitsSlice::from_slice` ignores its input and every accessor is
// unimplemented. It is kept out of the public API until it works, because most
// of its surface returns values rather than results and so cannot report that
// it did nothing.
#[allow(dead_code)]
mod fits_slice;
#[allow(dead_code)]
mod slice_ascii_table_hdu;
#[allow(dead_code)]
mod slice_bin_table_hdu;
#[allow(dead_code)]
mod slice_image_hdu;

pub use self::error::Error;
pub use self::fits::Fits;
pub use self::result::Result;
