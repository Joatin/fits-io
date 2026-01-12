#![no_std]
#![cfg_attr(nightly, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod ansi_table;
pub mod bin_table;
mod fits;
mod fits_slice;
#[cfg(feature = "fs")]
pub mod fs;
pub mod hdu;
pub mod header;
pub mod image;
mod slice_ascii_table_hdu;
mod slice_bin_table_hdu;
mod slice_image_hdu;
mod util;

pub use self::fits::Fits;
pub use self::fits_slice::FitsSlice;
