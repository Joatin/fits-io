use crate::hdu::{AsciiTableHDU, BinTableHDU, ExtensionHDU, ImageHDU};
use std::error::Error;
use std::fmt::Debug;

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

    /// Serialises this FITS file back into its on-disk byte representation.
    ///
    /// Writing is not implemented yet, so every backend currently returns an
    /// error rather than a partial file.
    fn to_vec(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>>;
}
