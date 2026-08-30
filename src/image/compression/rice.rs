//! Rice decompression, as the tiled image convention uses it.
//!
//! Rice coding stores the difference between each value and the one before it,
//! folded so that small negative differences are small numbers, and then splits
//! each difference into a low part written verbatim and a high part written as
//! that many zero bits. Data that changes slowly — which is most images — comes
//! out much smaller, and nothing is lost.

use std::error::Error;

/// How many bytes each value of the original image occupied.
///
/// The parameters of the coding depend on it, so a tile decoded at the wrong
/// width comes out as noise rather than as an error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BytesPerValue {
    One,
    Two,
    Four,
}

impl BytesPerValue {
    /// How many bits hold the block's split point.
    fn split_bits(self) -> u32 {
        match self {
            BytesPerValue::One => 3,
            BytesPerValue::Two => 4,
            BytesPerValue::Four => 5,
        }
    }

    /// The split point that means "this block is stored verbatim".
    fn max_split(self) -> u32 {
        match self {
            BytesPerValue::One => 6,
            BytesPerValue::Two => 14,
            BytesPerValue::Four => 25,
        }
    }

    /// How many bits one value occupies when stored verbatim.
    fn value_bits(self) -> u32 {
        8 * self as u32
    }

    fn bytes(self) -> usize {
        match self {
            BytesPerValue::One => 1,
            BytesPerValue::Two => 2,
            BytesPerValue::Four => 4,
        }
    }

    pub(crate) fn from_count(bytes: i64) -> Result<Self, Box<dyn Error + Send + Sync>> {
        match bytes {
            1 => Ok(BytesPerValue::One),
            2 => Ok(BytesPerValue::Two),
            4 => Ok(BytesPerValue::Four),
            other => Err(From::from(format!(
                "A Rice compressed tile is 1, 2 or 4 bytes per value, not {}",
                other
            ))),
        }
    }
}

/// Reads bits out of a byte slice, most significant first.
struct Bits<'a> {
    bytes: &'a [u8],
    at: usize,
    /// The bits not yet consumed, held in the low end of `buffer`.
    buffer: u32,
    count: u32,
}

impl<'a> Bits<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            at: 0,
            buffer: 0,
            count: 0,
        }
    }

    fn next_byte(&mut self) -> Result<u32, Box<dyn Error + Send + Sync>> {
        let byte = self
            .bytes
            .get(self.at)
            .ok_or("A Rice compressed tile ended before its values did")?;
        self.at += 1;

        Ok(*byte as u32)
    }

    /// Takes the next `wanted` bits.
    fn take(&mut self, wanted: u32) -> Result<u32, Box<dyn Error + Send + Sync>> {
        while self.count < wanted {
            let byte = self.next_byte()?;
            self.buffer = (self.buffer << 8) | byte;
            self.count += 8;
        }

        self.count -= wanted;
        let value = self.buffer >> self.count;
        self.buffer &= (1_u32 << self.count) - 1;

        Ok(value)
    }

    /// Counts zero bits up to and including the next one bit.
    fn zeros_before_one(&mut self) -> Result<u32, Box<dyn Error + Send + Sync>> {
        let mut zeros = 0;

        loop {
            if self.buffer == 0 {
                // Every bit held is zero, so they all count and more are needed.
                zeros += self.count;
                self.buffer = self.next_byte()?;
                self.count = 8;
                continue;
            }

            // The highest set bit ends the run.
            let highest = u32::BITS - self.buffer.leading_zeros();
            zeros += self.count - highest;

            self.count = highest - 1;
            self.buffer &= (1_u32 << self.count) - 1;

            return Ok(zeros);
        }
    }
}

/// Undoes the folding that made small negative differences small numbers.
fn unfold(value: u32) -> i64 {
    if value & 1 == 0 {
        (value >> 1) as i64
    } else {
        !((value >> 1) as i64)
    }
}

/// Decompresses one Rice-coded tile into `count` values.
///
/// The values come back widened to `i64`; the caller knows from ZBITPIX what
/// type they started as.
pub(crate) fn decompress(
    bytes: &[u8],
    count: usize,
    width: BytesPerValue,
    block: usize,
) -> Result<Vec<i64>, Box<dyn Error + Send + Sync>> {
    if count == 0 {
        return Ok(Vec::new());
    }

    let block = block.max(1);

    // The first value is stored plainly at the front, and everything after it is
    // a difference from the one before.
    let header = bytes
        .get(..width.bytes())
        .ok_or("A Rice compressed tile is too short to hold its first value")?;

    let mut previous = header
        .iter()
        .fold(0_u32, |value, byte| (value << 8) | *byte as u32);

    let mut bits = Bits::new(&bytes[width.bytes()..]);
    let mut values = Vec::with_capacity(count);

    while values.len() < count {
        let split = bits.take(width.split_bits())?;
        let remaining = (count - values.len()).min(block);

        // A split of zero marks a run of identical values, and the maximum marks
        // a block the coder gave up on and stored verbatim.
        if split == 0 {
            for _ in 0..remaining {
                values.push(previous);
            }
            continue;
        }

        let split = split - 1;

        for _ in 0..remaining {
            let difference = if split == width.max_split() {
                bits.take(width.value_bits())?
            } else {
                let high = bits.zeros_before_one()?;
                let low = if split > 0 { bits.take(split)? } else { 0 };

                (high << split) | low
            };

            previous = previous.wrapping_add(unfold(difference) as u32);
            values.push(previous);
        }
    }

    // Widen back to signed values of the original type.
    Ok(values
        .into_iter()
        .map(|value| match width {
            BytesPerValue::One => value as u8 as i64,
            BytesPerValue::Two => value as u16 as i16 as i64,
            BytesPerValue::Four => value as i32 as i64,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{BytesPerValue, decompress};

    /// Every vector here was produced by cfitsio, through astropy's
    /// `CompImageHDU`, so these check this decoder against the implementation
    /// the convention was written around rather than against itself.
    #[test]
    fn eight_bit_values_match_the_reference_implementation() {
        let compressed = [0x03, 0xd0, 0x41, 0x05, 0x97, 0x8c, 0x47, 0xd0, 0x00];

        let values = decompress(&compressed, 8, BytesPerValue::One, 32).expect("a valid tile");

        assert_eq!(values, vec![3, 3, 3, 9, 1, 200, 7, 7]);
    }

    #[test]
    fn sixteen_bit_values_match_the_reference_implementation() {
        // Opens with four identical values, which the coder writes as a run.
        let compressed = [
            0xff, 0xfb, 0x88, 0x08, 0x08, 0x08, 0x06, 0x90, 0x13, 0xe1, 0xcd, 0x00,
        ];

        let values = decompress(&compressed, 8, BytesPerValue::Two, 32).expect("a valid tile");

        assert_eq!(values, vec![-5, -5, -5, -5, 100, -300, 7, 7]);
    }

    #[test]
    fn thirty_two_bit_values_match_the_reference_implementation() {
        let compressed = [
            0x00, 0x00, 0x00, 0x00, 0x8c, 0x00, 0x02, 0x00, 0x05, 0x00, 0x03, 0x24, 0x5c, 0x41,
            0x45, 0xbf, 0x24, 0x5d, 0x50, 0x00, 0x08, 0x00, 0x00,
        ];

        let values = decompress(&compressed, 8, BytesPerValue::Four, 32).expect("a valid tile");

        assert_eq!(values, vec![0, 1, -1, 70000, -70000, 5, 5, 5]);
    }

    #[test]
    fn a_whole_tile_of_a_gradient_decodes() {
        // Four rows of sixteen, values i * 7 - j * 3, in one tile.
        let compressed = [
            0x00, 0x00, 0x58, 0x7b, 0xde, 0xf7, 0xbd, 0xef, 0x7b, 0xde, 0xf7, 0xbd, 0xe0, 0x00,
            0x5f, 0xde, 0xf7, 0xbd, 0xef, 0x7b, 0xde, 0xf7, 0xbd, 0xef, 0x28, 0x00, 0x2f, 0xef,
            0x7b, 0xde, 0xf7, 0xbd, 0xef, 0x7b, 0xde, 0xf7, 0x80, 0x01, 0x7f, 0x7b, 0xde, 0xf7,
            0xbd, 0xef, 0x7b, 0xde, 0xf7, 0xbc,
        ];

        let values = decompress(&compressed, 64, BytesPerValue::Two, 32).expect("a valid tile");

        let expected: Vec<i64> = (0..4)
            .flat_map(|row| (0..16).map(move |column| column * 7 - row * 3))
            .collect();

        assert_eq!(values, expected);
    }

    #[test]
    fn a_truncated_tile_is_an_error() {
        // Running out of bits must say so rather than inventing values.
        let error = decompress(&[0x00, 0x00, 0x58], 64, BytesPerValue::Two, 32)
            .expect_err("the tile ends before its values do");

        assert!(error.to_string().contains("ended before"), "got: {error}");
    }

    #[test]
    fn a_tile_of_no_values_decodes_to_nothing() {
        assert!(
            decompress(&[], 0, BytesPerValue::Two, 32)
                .expect("no values needs no data")
                .is_empty()
        );
    }
}
