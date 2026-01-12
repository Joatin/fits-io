use crate::Fits;

#[derive(Debug, Clone)]
pub enum ExtensionHDU<F: Fits> {
    Image(F::ImageHDU),
    BinTable(F::BinTableHDU),
    AsciiTable(F::AsciiTableHDU),
}
