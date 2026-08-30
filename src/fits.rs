use crate::hdu::{AsciiTableHDU, BinTableHDU, ExtensionHDU, ImageHDU};
use std::error::Error;
use std::fmt::Debug;

/// This is a representation of a FITS file (Flexible Image Transport System).
pub trait Fits: Debug + Clone {
    /// The kind of image HDU this reader produces.
    type ImageHDU: ImageHDU;
    /// The kind of binary table HDU this reader produces.
    type BinTableHDU: BinTableHDU;
    /// The kind of ASCII table HDU this reader produces.
    type AsciiTableHDU: AsciiTableHDU;

    /// The primary HDU, which every FITS file has.
    fn primary_hdu(&self) -> &Self::ImageHDU;
    /// The primary HDU, to be changed.
    fn primary_hdu_mut(&mut self) -> &mut Self::ImageHDU;

    /// How many extensions follow the primary HDU.
    fn extension_count(&self) -> usize;
    /// The extension at `index`, or `None` if there is none.
    fn extension_hdu(&self, index: usize) -> Option<&ExtensionHDU<Self>>;
    /// The extension at `index`, to be changed.
    fn extension_hdu_mut(&mut self, index: usize) -> Option<&mut ExtensionHDU<Self>>;
    /// Every extension, in the order they appear in the file.
    fn extension_hdus(&self) -> impl Iterator<Item = &ExtensionHDU<Self>>;
    /// Every extension, to be changed.
    fn extension_hdus_mut(&mut self) -> impl Iterator<Item = &mut ExtensionHDU<Self>>;

    /// Appends an extension after the ones already here.
    fn push_extension(&mut self, extension: ExtensionHDU<Self>);

    /// Removes the extension at `index`, or returns `None` if there is none.
    fn remove_extension(&mut self, index: usize) -> Option<ExtensionHDU<Self>>;

    /// Serialises this FITS file back into its on-disk byte representation.
    ///
    /// Writing is not implemented yet, so every backend currently returns an
    /// error rather than a partial file.
    fn to_vec(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>>;
}
