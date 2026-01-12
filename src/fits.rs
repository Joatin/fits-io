use crate::hdu::{AsciiTableHDU, BinTableHDU, ExtensionHDU, ImageHDU};
use alloc::vec::Vec;
use core::fmt::Debug;

/// This is a representation of a FITS file (Flexible Image Transport System).
pub trait Fits: Debug + Clone {
    type ImageHDU: ImageHDU;
    type BinTableHDU: BinTableHDU;
    type AsciiTableHDU: AsciiTableHDU;

    fn primary_hdu(&self) -> &Self::ImageHDU;
    fn primary_hdu_mut(&mut self) -> &mut Self::ImageHDU;

    fn extension_count(&mut self) -> usize;
    fn extension_hdu(&self, index: usize) -> Option<&ExtensionHDU<Self>>;
    fn extension_hdu_mut(&mut self, index: usize) -> Option<&mut ExtensionHDU<Self>>;
    fn extension_hdus(&self) -> impl Iterator<Item = &ExtensionHDU<Self>>;
    fn extension_hdus_mut(&mut self) -> impl Iterator<Item = &mut ExtensionHDU<Self>>;

    fn to_vec(&self) -> Vec<u8>;
}
