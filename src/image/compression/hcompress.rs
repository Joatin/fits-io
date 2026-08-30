//! HCOMPRESS decompression, as the tiled image convention uses it.
//!
//! HCOMPRESS was written by Richard White at the Space Telescope Science
//! Institute and used for the Digitized Sky Survey. It works in three stages,
//! which this undoes in reverse:
//!
//! 1. An **H-transform**, a Haar wavelet generalised to two dimensions, which
//!    replaces each 2x2 block by its sum and three differences and then repeats
//!    on the sums. It is exactly reversible in integer arithmetic.
//! 2. **Quantisation**, dividing the coefficients by a scale factor. A scale of
//!    0 or 1 keeps everything and the whole thing is lossless.
//! 3. **Quadtree coding** of each bit plane of the result, with a fixed Huffman
//!    code over the four-bit quadtree nodes.
//!
//! The convention specifies the first two stages only in prose and refers to
//! White's papers for the third, so the bit-level details here were taken from
//! the reference implementation's algorithm and checked against what it
//! produces.

use std::error::Error;

/// The two bytes every compressed tile begins with.
const MAGIC: [u8; 2] = [0xdd, 0x99];

/// Reads the bits of a compressed tile.
///
/// Bits come out most significant first. The reader is restarted between the
/// bit planes and the sign bits, which is why `restart` exists.
struct Bits<'a> {
    bytes: &'a [u8],
    at: usize,
    buffer: u32,
    count: u32,
}

impl<'a> Bits<'a> {
    fn new(bytes: &'a [u8], at: usize) -> Self {
        Self {
            bytes,
            at,
            buffer: 0,
            count: 0,
        }
    }

    /// Drops any part-read byte, so the next read starts on a byte boundary.
    fn restart(&mut self) {
        self.count = 0;
    }

    fn byte(&mut self) -> Result<u32, Box<dyn Error + Send + Sync>> {
        let byte = *self
            .bytes
            .get(self.at)
            .ok_or("An HCOMPRESS tile ended before its values did")?;
        self.at += 1;

        Ok(byte as u32)
    }

    fn bit(&mut self) -> Result<u32, Box<dyn Error + Send + Sync>> {
        if self.count == 0 {
            self.buffer = self.byte()?;
            self.count = 8;
        }

        self.count -= 1;

        Ok((self.buffer >> self.count) & 1)
    }

    /// Takes `n` bits, where `n` is at most 8.
    fn take(&mut self, n: u32) -> Result<u32, Box<dyn Error + Send + Sync>> {
        if self.count < n {
            let byte = self.byte()?;
            // Only the low bits are ever read back, so the buffer is kept small
            // rather than letting the shifted-out bits accumulate.
            self.buffer = ((self.buffer << 8) | byte) & 0xFFFF;
            self.count += 8;
        }

        self.count -= n;

        Ok((self.buffer >> self.count) & ((1 << n) - 1))
    }

    fn nybble(&mut self) -> Result<u32, Box<dyn Error + Send + Sync>> {
        self.take(4)
    }

    /// Reads one quadtree node under the fixed Huffman code.
    ///
    /// The four-bit node values are not equally likely — a node with one corner
    /// set is far commoner than one with three — so the common ones are given
    /// three-bit codes and the rest four, five or six.
    fn huffman(&mut self) -> Result<u32, Box<dyn Error + Send + Sync>> {
        let code = self.take(3)?;
        if code < 4 {
            // 1, 2, 4 and 8: the nodes with a single corner set.
            return Ok(1 << code);
        }

        let code = self.bit()? | (code << 1);
        if code < 13 {
            return Ok(match code {
                8 => 3,
                9 => 5,
                10 => 10,
                11 => 12,
                _ => 15,
            });
        }

        let code = self.bit()? | (code << 1);
        if code < 31 {
            return Ok(match code {
                26 => 6,
                27 => 7,
                28 => 9,
                29 => 11,
                _ => 13,
            });
        }

        let code = self.bit()? | (code << 1);

        Ok(if code == 62 { 0 } else { 14 })
    }
}

/// The number of doublings needed to reach `value`, which is how many times a
/// quadtree is expanded.
fn levels(value: usize) -> u32 {
    if value <= 1 {
        return 0;
    }

    usize::BITS - (value - 1).leading_zeros()
}

/// Decompresses one HCOMPRESS-coded tile.
///
/// Returns the values along with the tile's own idea of its shape, as rows and
/// columns, so that a caller can check them against the header.
pub(crate) fn decompress(
    bytes: &[u8],
    smooth: bool,
) -> Result<(Vec<i64>, usize, usize), Box<dyn Error + Send + Sync>> {
    if smooth {
        return Err("HCOMPRESS images that ask to be smoothed are not supported yet".into());
    }

    if bytes.get(..2) != Some(&MAGIC[..]) {
        return Err("This is not an HCOMPRESS compressed tile: the magic bytes are wrong".into());
    }

    let integer = |at: usize| -> Result<i64, Box<dyn Error + Send + Sync>> {
        let word = bytes
            .get(at..at + 4)
            .ok_or("An HCOMPRESS tile is too short to hold its header")?;

        Ok(i32::from_be_bytes([word[0], word[1], word[2], word[3]]) as i64)
    };

    // Rows and columns, in that order; the second is the one that varies
    // fastest, which is the image's width.
    let rows = integer(2)?;
    let columns = integer(6)?;
    let scale = integer(10)?;

    let sum = bytes
        .get(14..22)
        .ok_or("An HCOMPRESS tile is too short to hold its header")?;
    let sum = i64::from_be_bytes(sum.try_into().expect("eight bytes"));

    let planes = bytes
        .get(22..25)
        .ok_or("An HCOMPRESS tile is too short to hold its header")?;
    let planes = [planes[0] as u32, planes[1] as u32, planes[2] as u32];

    let (rows, columns) = match (usize::try_from(rows), usize::try_from(columns)) {
        (Ok(rows), Ok(columns)) if rows > 0 && columns > 0 => (rows, columns),
        _ => {
            return Err(format!("An HCOMPRESS tile of {}x{} makes no sense", rows, columns).into());
        }
    };

    let mut values = decode(&mut Bits::new(bytes, 25), rows, columns, planes)?;

    // The sum of everything is kept out of the coded planes and put back here.
    values[0] = sum;

    if scale > 1 {
        for value in values.iter_mut() {
            *value *= scale;
        }
    }

    inverse_transform(&mut values, rows, columns);

    Ok((values, rows, columns))
}

/// Reads the four quadrants' bit planes, then the sign of every non-zero value.
fn decode(
    bits: &mut Bits,
    rows: usize,
    columns: usize,
    planes: [u32; 3],
) -> Result<Vec<i64>, Box<dyn Error + Send + Sync>> {
    let mut values = vec![0_i64; rows * columns];

    let rows2 = rows.div_ceil(2);
    let columns2 = columns.div_ceil(2);

    // The transform leaves four quadrants of differing character, so each is
    // coded separately and the middle two share a bit plane count.
    let quadrants = [
        (0, rows2, columns2, planes[0]),
        (columns2, rows2, columns / 2, planes[1]),
        (columns * rows2, rows / 2, columns2, planes[1]),
        (columns * rows2 + columns2, rows / 2, columns / 2, planes[2]),
    ];

    for (offset, height, width, count) in quadrants {
        quadtree_decode(bits, &mut values, offset, columns, height, width, count)?;
    }

    if bits.nybble()? != 0 {
        return Err("An HCOMPRESS tile does not end where its bit planes say it should".into());
    }

    // The planes carry magnitudes; the signs follow them, one bit per value that
    // turned out to be non-zero.
    bits.restart();
    for value in values.iter_mut() {
        if *value != 0 && bits.bit()? == 1 {
            *value = -*value;
        }
    }

    Ok(values)
}

/// Reads one quadrant's bit planes into `values`.
fn quadtree_decode(
    bits: &mut Bits,
    values: &mut [i64],
    offset: usize,
    stride: usize,
    height: usize,
    width: usize,
    planes: u32,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if height == 0 || width == 0 {
        return Ok(());
    }

    let log2n = levels(height.max(width));

    let mut nodes = vec![0_u8; height.div_ceil(2) * width.div_ceil(2)];

    // The planes are read from the top down, most significant first.
    for plane in (0..planes).rev() {
        match bits.nybble()? {
            // A plane too dense to be worth coding is written out as it stands.
            0 => {
                for node in nodes
                    .iter_mut()
                    .take(height.div_ceil(2) * width.div_ceil(2))
                {
                    *node = bits.nybble()? as u8;
                }
                insert_plane(&nodes, height, width, values, offset, stride, plane);
            }

            0xf => {
                nodes[0] = bits.huffman()? as u8;

                // Each step doubles the resolution, reading a new node wherever
                // the previous level said there was something to see.
                let (mut nx, mut ny) = (1_usize, 1_usize);
                let (mut fx, mut fy) = (height, width);
                let mut c = 1_usize << log2n;

                for _ in 1..log2n {
                    c >>= 1;
                    nx <<= 1;
                    ny <<= 1;
                    if fx <= c {
                        nx -= 1;
                    } else {
                        fx -= c;
                    }
                    if fy <= c {
                        ny -= 1;
                    } else {
                        fy -= c;
                    }

                    expand(bits, &mut nodes, nx, ny)?;
                }

                insert_plane(&nodes, height, width, values, offset, stride, plane);
            }

            other => {
                return Err(format!(
                    "An HCOMPRESS bit plane is marked {}, which is neither coded nor plain",
                    other
                )
                .into());
            }
        }
    }

    Ok(())
}

/// Doubles the resolution of a quadtree level in place, reading a node for every
/// corner the level below said was set.
fn expand(
    bits: &mut Bits,
    nodes: &mut [u8],
    nx: usize,
    ny: usize,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    spread(nodes, nx, ny);

    for index in (0..nx * ny).rev() {
        if nodes[index] != 0 {
            nodes[index] = bits.huffman()? as u8;
        }
    }

    Ok(())
}

/// Spreads each four-bit node over the 2x2 block of corners it describes.
///
/// The bits run from the top left down: bit 3 is the top left corner, bit 0 the
/// bottom right.
fn spread(nodes: &mut [u8], nx: usize, ny: usize) {
    let nx2 = nx.div_ceil(2);
    let ny2 = ny.div_ceil(2);

    // Move each node out to where its block starts, from the end so that the
    // moves do not tread on nodes not yet read.
    for i in (0..nx2).rev() {
        for j in (0..ny2).rev() {
            nodes[2 * (ny * i + j)] = nodes[ny2 * i + j];
        }
    }

    let mut i = 0;
    while i + 1 < nx {
        let mut j = 0;
        while j + 1 < ny {
            let at = ny * i + j;
            let code = nodes[at];

            nodes[at + ny + 1] = code & 1;
            nodes[at + ny] = (code >> 1) & 1;
            nodes[at + 1] = (code >> 2) & 1;
            nodes[at] = (code >> 3) & 1;

            j += 2;
        }

        // An odd width leaves one corner of the block off the edge.
        if j < ny {
            let at = ny * i + j;
            let code = nodes[at];
            nodes[at + ny] = (code >> 1) & 1;
            nodes[at] = (code >> 3) & 1;
        }

        i += 2;
    }

    // An odd height leaves the bottom half of every block off the edge.
    if i < nx {
        let mut j = 0;
        while j + 1 < ny {
            let at = ny * i + j;
            let code = nodes[at];
            nodes[at + 1] = (code >> 2) & 1;
            nodes[at] = (code >> 3) & 1;
            j += 2;
        }

        if j < ny {
            let at = ny * i + j;
            nodes[at] = (nodes[at] >> 3) & 1;
        }
    }
}

/// Sets bit `plane` of each value the quadtree's leaves mark.
fn insert_plane(
    nodes: &[u8],
    height: usize,
    width: usize,
    values: &mut [i64],
    offset: usize,
    stride: usize,
    plane: u32,
) {
    let bit = 1_i64 << plane;
    let mut k = 0;

    let mut i = 0;
    while i + 1 < height {
        let mut at = offset + stride * i;

        let mut j = 0;
        while j + 1 < width {
            let code = nodes[k];

            if code & 1 != 0 {
                values[at + stride + 1] |= bit;
            }
            if code & 2 != 0 {
                values[at + stride] |= bit;
            }
            if code & 4 != 0 {
                values[at + 1] |= bit;
            }
            if code & 8 != 0 {
                values[at] |= bit;
            }

            k += 1;
            at += 2;
            j += 2;
        }

        if j < width {
            let code = nodes[k];
            if code & 2 != 0 {
                values[at + stride] |= bit;
            }
            if code & 8 != 0 {
                values[at] |= bit;
            }
            k += 1;
        }

        i += 2;
    }

    if i < height {
        let mut at = offset + stride * i;

        let mut j = 0;
        while j + 1 < width {
            let code = nodes[k];
            if code & 4 != 0 {
                values[at + 1] |= bit;
            }
            if code & 8 != 0 {
                values[at] |= bit;
            }
            k += 1;
            at += 2;
            j += 2;
        }

        if j < width && nodes[k] & 8 != 0 {
            values[at] |= bit;
        }
    }
}

/// Interleaves the halves of a run back together, undoing the shuffle that put
/// the sums before the differences.
fn unshuffle(values: &mut [i64], start: usize, n: usize, stride: usize, tmp: &mut Vec<i64>) {
    let half = n.div_ceil(2);

    tmp.clear();
    for i in half..n {
        tmp.push(values[start + stride * i]);
    }

    for i in (0..half).rev() {
        values[start + stride * 2 * i] = values[start + stride * i];
    }

    for (index, i) in (1..n).step_by(2).enumerate() {
        values[start + stride * i] = tmp[index];
    }
}

/// The inverse H-transform.
///
/// Each round interleaves the coefficients back into place and then rebuilds
/// every 2x2 block from its sum and three differences. The rounding constants
/// change on each round, and the last round shifts by two rather than one, which
/// is what makes the whole thing exactly reversible.
fn inverse_transform(values: &mut [i64], rows: usize, columns: usize) {
    if rows * columns <= 1 {
        return;
    }

    let log2n = levels(rows.max(columns));

    let mut shift = 1_u32;
    let mut bit0 = 1_i64 << log2n.saturating_sub(1);
    let mut bit1 = bit0 << 1;
    let mut mask0 = -bit0;
    let mut mask1 = mask0 << 1;
    let mask2 = mask0 << 2;
    let mut round0 = bit0 >> 1;
    let mut round1 = bit1 >> 1;
    let round2 = (bit0 << 2) >> 1;
    let mut negative0 = round0 - 1;
    let mut negative1 = round1 - 1;
    let negative2 = round2 - 1;

    values[0] = (values[0] + if values[0] >= 0 { round2 } else { negative2 }) & mask2;

    let (mut height, mut width) = (1_usize, 1_usize);
    let (mut fx, mut fy) = (rows, columns);
    let mut c = 1_usize << log2n;

    let mut tmp = Vec::with_capacity(rows.max(columns));

    for level in (0..log2n).rev() {
        c >>= 1;
        height <<= 1;
        width <<= 1;
        if fx <= c {
            height -= 1;
        } else {
            fx -= c;
        }
        if fy <= c {
            width -= 1;
        } else {
            fy -= c;
        }

        // The last round carries no rounding and shifts by two, because the
        // first round of the forward transform did not divide.
        if level == 0 {
            negative0 = 0;
            shift = 2;
        }

        for i in 0..height {
            unshuffle(values, columns * i, width, 1, &mut tmp);
        }
        for j in 0..width {
            unshuffle(values, j, height, columns, &mut tmp);
        }

        let odd_rows = height % 2;
        let odd_columns = width % 2;

        let mut i = 0;
        while i < height - odd_rows {
            let mut at = columns * i;

            let mut j = 0;
            while j < width - odd_columns {
                let h0 = values[at];
                let mut hx = values[at + columns];
                let mut hy = values[at + 1];
                let hc = values[at + columns + 1];

                let hc = (hc + if hc >= 0 { round0 } else { negative0 }) & mask0;
                hx = (hx + if hx >= 0 { round1 } else { negative1 }) & mask1;
                hy = (hy + if hy >= 0 { round1 } else { negative1 }) & mask1;

                // The bit the differences gave up to the sum is handed back.
                let low0 = hc & bit0;
                hx = if hx >= 0 { hx - low0 } else { hx + low0 };
                hy = if hy >= 0 { hy - low0 } else { hy + low0 };

                let low1 = (hc ^ hx ^ hy) & bit1;
                let h0 = if h0 >= 0 {
                    h0 + low0 - low1
                } else {
                    h0 + if low0 == 0 { low1 } else { low0 - low1 }
                };

                values[at + columns + 1] = (h0 + hx + hy + hc) >> shift;
                values[at + columns] = (h0 + hx - hy - hc) >> shift;
                values[at + 1] = (h0 - hx + hy - hc) >> shift;
                values[at] = (h0 - hx - hy + hc) >> shift;

                at += 2;
                j += 2;
            }

            // An odd width leaves a column with no partner.
            if odd_columns == 1 {
                let h0 = values[at];
                let hx = values[at + columns];
                let hx = (hx + if hx >= 0 { round1 } else { negative1 }) & mask1;
                let low1 = hx & bit1;
                let h0 = if h0 >= 0 { h0 - low1 } else { h0 + low1 };

                values[at + columns] = (h0 + hx) >> shift;
                values[at] = (h0 - hx) >> shift;
            }

            i += 2;
        }

        // An odd height leaves a row with no partner.
        if odd_rows == 1 {
            let mut at = columns * i;

            let mut j = 0;
            while j < width - odd_columns {
                let h0 = values[at];
                let hy = values[at + 1];
                let hy = (hy + if hy >= 0 { round1 } else { negative1 }) & mask1;
                let low1 = hy & bit1;
                let h0 = if h0 >= 0 { h0 - low1 } else { h0 + low1 };

                values[at + 1] = (h0 + hy) >> shift;
                values[at] = (h0 - hy) >> shift;

                at += 2;
                j += 2;
            }

            if odd_columns == 1 {
                values[at] >>= shift;
            }
        }

        // Each round works one bit lower than the last.
        bit1 = bit0;
        bit0 >>= 1;
        mask1 = mask0;
        mask0 >>= 1;
        round1 = round0;
        round0 >>= 1;
        negative1 = negative0;
        negative0 = round0 - 1;
    }
}

#[cfg(test)]
mod tests {
    use super::{decompress, levels};

    #[test]
    fn an_eight_by_eight_gradient_matches_the_reference_implementation() {
        // A ramp from 0 to 63. Every vector here was produced by cfitsio, through
        // astropy, so these check this decoder against the implementation the
        // convention refers to rather than against itself.
        let tile = [
            0xdd, 0x99, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x09, 0x05, 0x00, 0xf6, 0x7f, 0xef,
            0x39, 0xed, 0x7f, 0xde, 0xb3, 0xfe, 0xff, 0xbf, 0xef, 0xfb, 0xfe, 0xff, 0x83, 0xff,
            0xff, 0xfe, 0x0f, 0xff, 0xff, 0xfb, 0xfe, 0xff, 0xbf, 0xe0, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];

        let (values, rows, columns) = decompress(&tile, false).expect("a valid tile");

        assert_eq!((rows, columns), (8, 8));
        assert_eq!(values, (0..64).collect::<Vec<i64>>());
    }

    #[test]
    fn odd_dimensions_in_both_axes_match_the_reference_implementation() {
        // Five rows of seven. Both axes being odd exercises every edge case in the
        // quadtree expansion and in the inverse transform at once.
        let tile = [
            0xdd, 0x99, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x90, 0x09, 0x07, 0x08, 0xf4, 0x3f, 0xef,
            0xbf, 0xf4, 0x06, 0xf9, 0x80, 0xce, 0x08, 0x0e, 0x20, 0x9f, 0xf7, 0x87, 0xff, 0x7c,
            0x35, 0xee, 0xc3, 0x23, 0x00, 0x36, 0xa2, 0x02, 0x4b, 0x00, 0x3e, 0xa0, 0x3f, 0xef,
            0x76, 0x15, 0x00, 0xf2, 0x07, 0xe1, 0x64, 0x0b, 0xbf, 0xf7, 0xb0, 0x09, 0x01, 0x55,
            0xed, 0xfb, 0x90, 0x8a, 0x05, 0xa0, 0xfa, 0x00, 0x16, 0x47, 0x29, 0x71, 0x00,
        ];

        let (values, rows, columns) = decompress(&tile, false).expect("a valid tile");

        assert_eq!((rows, columns), (5, 7));
        assert_eq!(
            values,
            vec![
                90, 33, 8, 74, 14, 53, 64, 14, 10, 86, 77, 94, 30, 59, 44, 61, 69, 6, 30, 17, 61,
                31, 15, 14, 90, 16, 42, 74, 3, 18, 60, 18, 62, 98, 2
            ]
        );
    }

    #[test]
    fn a_tile_that_is_not_hcompress_is_rejected() {
        // Every compressed tile opens with the same two bytes; anything else is
        // not one, and decoding it would produce noise.
        let error = decompress(&[0x00, 0x00, 0x00, 0x00], false)
            .expect_err("these are not the magic bytes");

        assert!(error.to_string().contains("magic"), "got: {error}");
    }

    #[test]
    fn a_truncated_tile_is_an_error() {
        let error =
            decompress(&[0xdd, 0x99, 0x00, 0x00], false).expect_err("the header is not complete");

        assert!(error.to_string().contains("too short"), "got: {error}");
    }

    #[test]
    fn smoothing_reports_itself_as_unimplemented() {
        // Smoothing changes the pixels, so quietly skipping it would hand back
        // an image that differs from what the file asked for.
        let error = decompress(&[0xdd, 0x99], true).expect_err("smoothing is not implemented");

        assert!(error.to_string().contains("smoothed"), "got: {error}");
    }

    #[test]
    fn the_level_count_is_the_doublings_needed_to_reach_a_size() {
        // The quadtree is expanded this many times, so an off-by-one here
        // misreads every tile whose size is not a power of two.
        assert_eq!(levels(1), 0);
        assert_eq!(levels(2), 1);
        assert_eq!(levels(3), 2);
        assert_eq!(levels(4), 2);
        assert_eq!(levels(5), 3);
        assert_eq!(levels(8), 3);
        assert_eq!(levels(9), 4);
    }
}
