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
        // From the width in bytes, not from the variant's position: the two are
        // not the same number, and a block stored verbatim would be read at the
        // wrong width.
        8 * self.bytes() as u32
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
    ///
    /// Wider than the values it produces: a block stored verbatim is read a
    /// whole value at a time, and the bits of it that have not yet arrived on a
    /// byte boundary have to sit somewhere.
    buffer: u64,
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

    fn next_byte(&mut self) -> Result<u64, Box<dyn Error + Send + Sync>> {
        let byte = self
            .bytes
            .get(self.at)
            .ok_or("A Rice compressed tile ended before its values did")?;
        self.at += 1;

        Ok(*byte as u64)
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
        self.buffer &= (1_u64 << self.count) - 1;

        Ok(value as u32)
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
            let highest = u64::BITS - self.buffer.leading_zeros();
            zeros += self.count - highest;

            self.count = highest - 1;
            self.buffer &= (1_u64 << self.count) - 1;

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

/// Writes bits into a byte vector, most significant first.
struct BitWriter {
    bytes: Vec<u8>,
    /// The bits written into the byte being filled, held at its low end.
    buffer: u32,
    /// How many bits of the current byte are still free.
    free: u32,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            buffer: 0,
            free: 8,
        }
    }

    /// Writes the low `count` bits of `value`.
    fn write(&mut self, value: u32, count: u32) {
        let mut left = count;

        while left > 0 {
            let take = left.min(self.free);
            let bits = (value >> (left - take)) & ((1_u32 << take) - 1);

            self.buffer = (self.buffer << take) | bits;
            self.free -= take;
            left -= take;

            if self.free == 0 {
                self.bytes.push(self.buffer as u8);
                self.buffer = 0;
                self.free = 8;
            }
        }
    }

    /// Writes `count` zero bits followed by a one, which is how a Rice code
    /// spells the high part of a difference.
    fn write_zeros_then_one(&mut self, count: u32) {
        let mut left = count;

        // A run longer than a word is written a word at a time.
        while left >= 32 {
            self.write(0, 32);
            left -= 32;
        }

        self.write(1, left + 1);
    }

    /// The bytes written, with the last one padded out with zeros.
    fn finish(mut self) -> Vec<u8> {
        if self.free < 8 {
            self.bytes.push((self.buffer << self.free) as u8);
        }

        self.bytes
    }
}

/// Folds a difference into the non-negative number the coder writes.
///
/// Small differences of either sign have to come out as small numbers, so the
/// sign goes into the lowest bit rather than the highest: 0, -1, 1, -2 become
/// 0, 1, 2, 3.
fn fold(difference: i64) -> u32 {
    if difference < 0 {
        !((difference << 1) as u32)
    } else {
        (difference << 1) as u32
    }
}

/// Compresses `values` into a Rice-coded tile.
///
/// The values are taken as being of `width`, and anything they carry above that
/// is dropped: a tile is compressed as the type its image is stored in.
pub(crate) fn compress(values: &[i64], width: BytesPerValue, block: usize) -> Vec<u8> {
    if values.is_empty() {
        return Vec::new();
    }

    let block = block.max(1);
    let bits = width.value_bits();

    // The differences are taken in the arithmetic of the image's own type, so
    // that they wrap the same way the decoder's do.
    let truncate = |value: i64| -> i64 {
        let shift = 64 - bits;
        (value << shift) >> shift
    };

    let mut out = BitWriter::new();

    // The first value is written whole, and everything after it is a difference
    // from the one before.
    let first = truncate(values[0]) as u64 as u32 & mask(bits);
    out.write(first, bits);

    let mut previous = truncate(values[0]);
    let mut differences = Vec::with_capacity(block);

    for chunk in values.chunks(block) {
        differences.clear();
        let mut total = 0.0_f64;

        for value in chunk {
            let value = truncate(*value);
            let folded = fold(truncate(value.wrapping_sub(previous)));

            total += folded as f64;
            differences.push(folded);
            previous = value;
        }

        // The split point that makes the codes shortest: the mean difference
        // says how many low bits are worth writing verbatim, and the rest of
        // each difference is written as that many zeros.
        let mean = (total - (chunk.len() / 2) as f64 - 1.0) / chunk.len() as f64;
        let mut remaining = if mean < 0.0 { 0_u32 } else { mean as u32 >> 1 };

        let mut split = 0_u32;
        while remaining > 0 {
            remaining >>= 1;
            split += 1;
        }

        if split >= width.max_split() {
            // The differences are as large as the values, so coding them would
            // make the tile bigger; they go in whole instead.
            out.write(width.max_split() + 1, width.split_bits());
            for difference in &differences {
                out.write(*difference, bits);
            }
        } else if split == 0 && total == 0.0 {
            // Every value in the block is the one before it, which the decoder
            // reads from the split alone.
            out.write(0, width.split_bits());
        } else {
            out.write(split + 1, width.split_bits());

            for difference in &differences {
                out.write_zeros_then_one(difference >> split);
                if split > 0 {
                    out.write(difference & mask(split), split);
                }
            }
        }
    }

    out.finish()
}

/// The low `bits` bits set.
fn mask(bits: u32) -> u32 {
    if bits >= 32 {
        u32::MAX
    } else {
        (1 << bits) - 1
    }
}

#[cfg(test)]
mod tests {
    use super::{BytesPerValue, compress, decompress};

    /// The values of the "noisy" reference vector: a linear congruential
    /// sequence, whose differences are as large as the values themselves and so
    /// take the coder's high entropy path.
    fn noisy() -> Vec<i64> {
        let mut seed = 1_u32;

        (0..24)
            .map(|_| {
                seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
                (seed >> 3) as i64
            })
            .collect()
    }

    /// Each of these is the byte stream cfitsio produced for the same values,
    /// so this checks the encoder against the implementation the convention was
    /// written around rather than against this crate's own decoder.
    #[test]
    fn the_encoder_writes_the_bytes_the_reference_implementation_writes() {
        let smooth: Vec<i64> = (0..40).map(|i: i64| 1000 + i * i - (i % 7) * 13).collect();

        assert_eq!(
            compress(&smooth, BytesPerValue::Four, 32),
            vec![
                0x00, 0x00, 0x03, 0xe8, 0x34, 0x1b, 0xe7, 0x7d, 0x73, 0xc6, 0x0d, 0xa4, 0xa2, 0xcc,
                0x34, 0xe0, 0x0c, 0x90, 0x24, 0x50, 0xb1, 0x83, 0x40, 0x17, 0x3c, 0x20, 0x24, 0x28,
                0x2c, 0x30, 0x00, 0xa8, 0xe0, 0xf0, 0x40, 0x8e, 0x4e, 0x8e, 0xc2, 0x9b, 0xd3, 0xe3,
                0xf1, 0x00,
            ]
        );

        // A block of identical values, which the coder writes as a split of
        // zero and nothing else at all.
        assert_eq!(
            compress(&[42; 16], BytesPerValue::Four, 32),
            vec![0x00, 0x00, 0x00, 0x2a, 0x00]
        );

        let short: Vec<i64> = (0..20).map(|i: i64| -300 + i * 37).collect();
        assert_eq!(
            compress(&short, BytesPerValue::Two, 32),
            vec![
                0xfe, 0xd4, 0x78, 0x09, 0x49, 0x49, 0x49, 0x49, 0x49, 0x49, 0x49, 0x49, 0x49, 0x49,
                0x49, 0x49, 0x49, 0x49, 0x49, 0x49, 0x49, 0x49, 0x40,
            ]
        );

        let byte: Vec<i64> = (0..18).map(|i: i64| i * 5 - 40).collect();
        assert_eq!(
            compress(&byte, BytesPerValue::One, 32),
            vec![
                0xd8, 0x90, 0xa5, 0x29, 0x4a, 0x52, 0x94, 0xa5, 0x29, 0x4a, 0x52, 0x94, 0xa0,
            ]
        );
    }

    #[test]
    fn values_with_nothing_in_common_are_written_whole() {
        // When the differences are no smaller than the values, coding them
        // would make the tile bigger, and the coder writes them as they are.
        assert_eq!(
            compress(&noisy(), BytesPerValue::Four, 32),
            vec![
                0x08, 0x38, 0xcf, 0xd4, 0xd0, 0x00, 0x00, 0x00, 0x00, 0xa9, 0x70, 0x64, 0x80, 0xdd,
                0xf9, 0x98, 0x99, 0x39, 0xd3, 0x6d, 0x50, 0x69, 0xc0, 0x87, 0xf0, 0xc6, 0xa0, 0xd5,
                0x58, 0x87, 0xce, 0x83, 0x00, 0x79, 0xce, 0x0f, 0x88, 0x44, 0xb2, 0x1c, 0x81, 0x5e,
                0xbb, 0xcd, 0x38, 0x0a, 0x73, 0x47, 0x58, 0x4e, 0x45, 0x85, 0x90, 0x1c, 0xc7, 0xa4,
                0xc9, 0x17, 0x41, 0x9c, 0xe0, 0x62, 0x77, 0x57, 0xb8, 0x8d, 0x4d, 0x4c, 0xb0, 0x3a,
                0xf3, 0xe3, 0xc0, 0x1e, 0x84, 0x76, 0xf8, 0x1e, 0xe3, 0xf1, 0xe0, 0x03, 0x5a, 0x65,
                0xd0, 0x0a, 0xef, 0x56, 0x70, 0xab, 0xc7, 0xe8, 0xd8, 0x86, 0xc5, 0xca, 0x78, 0x4f,
                0x90, 0x0f, 0x08,
            ]
        );
    }

    #[test]
    fn what_the_encoder_writes_the_decoder_reads_back() {
        let cases: Vec<(Vec<i64>, BytesPerValue)> = vec![
            ((0..100).map(|i| i as i64).collect(), BytesPerValue::Four),
            (noisy(), BytesPerValue::Four),
            (vec![7; 70], BytesPerValue::Four),
            (
                (0..64).map(|i: i64| -30000 + i * 941).collect(),
                BytesPerValue::Two,
            ),
            (
                (0..37).map(|i: i64| (i * 11) % 256).collect(),
                BytesPerValue::One,
            ),
            (vec![0], BytesPerValue::Four),
        ];

        for (values, width) in cases {
            for block in [16, 32] {
                let compressed = compress(&values, width, block);
                let back = decompress(&compressed, values.len(), width, block)
                    .expect("what this crate wrote, it can read");

                assert_eq!(back, values, "{width:?} in blocks of {block}");
            }
        }
    }

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
