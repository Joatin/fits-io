#![cfg_attr(nightly, feature(doc_cfg))]
// `header::header::Header` and friends: the inner module is an implementation
// detail that the parent re-exports.
#![allow(clippy::module_inception)]
// Every public item is documented, and stays that way: an undocumented one is a
// build failure rather than a warning nobody reads.
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

pub mod ascii_table;
pub mod bin_table;
pub mod checksum;
mod error;
mod fits;
#[cfg(feature = "fs")]
/// Reading and writing FITS files on the filesystem.
pub mod fs;
/// The header and data units a FITS file is made of.
pub mod hdu;
pub mod header;
pub mod image;
mod result;
mod util;
pub mod wcs;

mod fits_slice;
mod slice_ascii_table_hdu;
mod slice_bin_table_hdu;
mod slice_image_hdu;

pub use self::error::Error;
pub use self::fits::Fits;
pub use self::fits_slice::FitsSlice;
pub use self::result::Result;
pub use self::slice_ascii_table_hdu::SliceAsciiTableHDU;
pub use self::slice_bin_table_hdu::SliceBinTableHDU;
pub use self::slice_image_hdu::SliceImageHDU;
