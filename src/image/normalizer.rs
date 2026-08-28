use crate::header::{Bitpix, Header};
use std::error::Error;

/// Maps raw FITS array values onto the `0.0..=1.0` range.
///
/// A FITS data array stores raw values that have to be shifted and scaled into
/// physical units before they mean anything:
///
/// ```text
/// physical = BZERO + BSCALE * raw
/// ```
///
/// Normalising additionally needs to know which physical values correspond to
/// black and white. Those come from the DATAMIN and DATAMAX cards when the file
/// carries them, and otherwise from the full representable range of BITPIX — for
/// example BITPIX = 16 with BZERO = 32768 describes unsigned 16-bit samples, so
/// physical 0 maps to 0.0 and physical 65535 maps to 1.0.
///
/// Floating point images have no representable range to fall back on, so they
/// require DATAMIN and DATAMAX; see [`Normalizer::from_header`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Normalizer {
    zero_offset: f64,
    scale: f64,
    minimum: f64,
    maximum: f64,
}

impl Normalizer {
    /// Builds a normaliser from explicit black and white points, given in
    /// physical units.
    pub fn new(zero_offset: f64, scale: f64, minimum: f64, maximum: f64) -> Self {
        // A negative BSCALE flips the ends of the range.
        Self {
            zero_offset,
            scale,
            minimum: minimum.min(maximum),
            maximum: minimum.max(maximum),
        }
    }

    /// Builds a normaliser spanning the full representable range of `bitpix`.
    ///
    /// Returns `None` for the floating point types, which have no such range;
    /// use [`Normalizer::from_samples`] or [`Normalizer::new`] for those.
    pub fn for_bitpix(bitpix: Bitpix, zero_offset: f64, scale: f64) -> Option<Self> {
        let (raw_min, raw_max) = bitpix.value_range()?;
        let physical = |raw: f64| zero_offset + scale * raw;

        Some(Self::new(
            zero_offset,
            scale,
            physical(raw_min),
            physical(raw_max),
        ))
    }

    /// Builds a normaliser whose black and white points are the smallest and
    /// largest values actually present in `samples`.
    ///
    /// This is the honest choice for floating point images, which carry no
    /// representable range — but it needs every sample up front, so it is only
    /// available once the whole array has been read.
    pub fn from_samples(
        zero_offset: f64,
        scale: f64,
        samples: impl IntoIterator<Item = f64>,
    ) -> Self {
        let mut minimum = f64::INFINITY;
        let mut maximum = f64::NEG_INFINITY;

        for raw in samples {
            let physical = zero_offset + scale * raw;
            if physical.is_finite() {
                minimum = minimum.min(physical);
                maximum = maximum.max(physical);
            }
        }

        // An empty or wholly non-finite array leaves no range to speak of.
        if !minimum.is_finite() || !maximum.is_finite() {
            minimum = 0.0;
            maximum = 0.0;
        }

        Self::new(zero_offset, scale, minimum, maximum)
    }

    /// Builds a normaliser from a header's BITPIX, BZERO, BSCALE, DATAMIN and
    /// DATAMAX cards.
    ///
    /// # Errors
    ///
    /// Returns an error for a floating point image (BITPIX -32 or -64) that
    /// carries neither DATAMIN nor DATAMAX. Such an image has no black and white
    /// point that can be known without reading every pixel, so a single-pass
    /// normalisation is not possible — read the image with
    /// [`ImageHDU::read_image`](crate::hdu::ImageHDU::read_image) instead, which
    /// has the whole array available.
    pub fn from_header(header: &Header) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let bitpix = header
            .bitpix()
            .ok_or("Cannot normalise an image from a header without a BITPIX card")?;

        let zero_offset = header.bzero_or_default();
        let scale = header.bscale_or_default();

        // DATAMIN and DATAMAX are already in physical units.
        if let (Some(minimum), Some(maximum)) = (header.data_min(), header.data_max()) {
            return Ok(Self::new(zero_offset, scale, minimum, maximum));
        }

        Self::for_bitpix(bitpix, zero_offset, scale).ok_or_else(|| {
            format!(
                "Cannot normalise a {:?} image in a single pass: it carries neither a DATAMIN nor \
                 a DATAMAX card, so its black and white points are unknown",
                bitpix
            )
            .into()
        })
    }

    /// Converts a raw array value into physical units.
    pub fn physical(&self, raw: f64) -> f64 {
        self.zero_offset + self.scale * raw
    }

    /// Converts a raw array value into the `0.0..=1.0` range.
    ///
    /// Values outside the black and white points are clamped, which matters when
    /// DATAMIN and DATAMAX do not actually bound the data.
    pub fn normalize(&self, raw: f64) -> f64 {
        let range = self.maximum - self.minimum;

        // A degenerate range (BSCALE of 0, or DATAMIN == DATAMAX) has no
        // gradient to spread values over.
        if range <= 0.0 || !range.is_finite() {
            return 0.0;
        }

        ((self.physical(raw) - self.minimum) / range).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Normalizer;

    fn normalizer(zero_offset: f64, scale: f64, minimum: f64, maximum: f64) -> Normalizer {
        Normalizer {
            zero_offset,
            scale,
            minimum,
            maximum,
        }
    }

    #[test]
    fn unsigned_16_bit_samples_span_the_full_range() {
        // BITPIX = 16 with BZERO = 32768 is the usual unsigned-short encoding.
        let normalizer = normalizer(32768.0, 1.0, 0.0, 65535.0);

        assert_eq!(normalizer.normalize(i16::MIN as f64), 0.0);
        assert_eq!(normalizer.normalize(i16::MAX as f64), 1.0);
        assert!((normalizer.normalize(-1.0) - 0.5).abs() < 1e-4);
    }

    #[test]
    fn raw_values_are_converted_to_physical_units() {
        let normalizer = normalizer(32768.0, 2.0, 0.0, 65535.0);

        assert_eq!(normalizer.physical(0.0), 32768.0);
        assert_eq!(normalizer.physical(10.0), 32788.0);
    }

    #[test]
    fn values_outside_the_range_are_clamped() {
        let normalizer = normalizer(0.0, 1.0, 10.0, 20.0);

        assert_eq!(normalizer.normalize(5.0), 0.0);
        assert_eq!(normalizer.normalize(25.0), 1.0);
        assert_eq!(normalizer.normalize(15.0), 0.5);
    }

    #[test]
    fn a_degenerate_range_does_not_produce_nan_or_infinity() {
        let normalizer = normalizer(0.0, 0.0, 7.0, 7.0);

        assert_eq!(normalizer.normalize(1.0), 0.0);
        assert_eq!(normalizer.normalize(f64::MAX), 0.0);
    }
}
