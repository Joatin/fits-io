use crate::Fits;

/// An extension HDU, which is one of the three kinds FITS defines.
#[derive(Debug, Clone)]
pub enum ExtensionHDU<F: Fits> {
    /// An image, or a stack of them.
    Image(F::ImageHDU),
    /// A binary table.
    BinTable(F::BinTableHDU),
    /// A table stored as fixed-width text.
    AsciiTable(F::AsciiTableHDU),
}
