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

    inverse_transform(&mut values, rows, columns, smooth, scale);

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
fn inverse_transform(values: &mut [i64], rows: usize, columns: usize, smooth: bool, scale: i64) {
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

        // Smoothing works on the coefficients of this level, once they are
        // interleaved and before they are turned back into pixels.
        if smooth {
            self::smooth(values, height, width, columns, scale);
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

/// Softens the artefacts lossy HCOMPRESS leaves, as the SMOOTH flag asks for.
///
/// Dividing the transform's coefficients by a scale factor throws away the low
/// bits of each one, and what is left is a coefficient that could have come from
/// anything within half a step of it. Smoothing picks, out of that range, the
/// value that best matches the neighbouring zones — so a gradient that the
/// coding flattened into blocks is bent back into a gradient.
///
/// Three passes, one for each kind of coefficient the transform produces: the
/// difference along each axis, and the curvature. Each is nudged towards the
/// slope its neighbours imply, by no more than half the scale factor, and only
/// where doing so does not overshoot the neighbouring values themselves — a
/// smoothed image must not invent a peak that the data does not support. The
/// coefficients around the edge have no neighbours on one side and are left
/// alone.
fn smooth(values: &mut [i64], height: usize, width: usize, columns: usize, scale: i64) {
    // Rounding during the division moved each coefficient by at most half a
    // step, so that is as far as it may be moved back.
    let limit = scale >> 1;
    if limit <= 0 {
        return;
    }

    let columns2 = columns * 2;

    /// The nudge that brings `current` to `wanted`, once divided by `2^bits`
    /// and held within `limit`.
    ///
    /// The shift stands in for a division, and a negative numerator is rounded
    /// the way the division would round it rather than the way the shift does.
    fn nudge(wanted: i64, current: i64, bits: u32, limit: i64) -> i64 {
        let difference = wanted - (current << bits);
        let scaled = if difference >= 0 {
            difference >> bits
        } else {
            (difference + (1 << bits) - 1) >> bits
        };

        scaled.clamp(-limit, limit)
    }

    /// How far the neighbours allow a coefficient to move, as the smallest and
    /// largest slopes that keep the zone between them.
    fn bounds(previous: i64, here: i64, next: i64) -> (i64, i64) {
        let rising = next - here;
        let falling = here - previous;

        (
            rising.min(falling).max(0) << 2,
            rising.max(falling).min(0) << 2,
        )
    }

    // The difference along the first axis, which is corrected from the mean
    // values of the zones on either side of it.
    let mut i = 2;
    while i + 2 < height {
        let mut at = columns * i;

        let mut j = 0;
        while j < width {
            let (upper, lower) = bounds(values[at - columns2], values[at], values[at + columns2]);

            // Neighbours that allow no slope at all leave the coefficient as it
            // is: there is nothing to interpolate between.
            if lower < upper {
                let wanted = (values[at + columns2] - values[at - columns2]).clamp(lower, upper);
                values[at + columns] += nudge(wanted, values[at + columns], 3, limit);
            }

            at += 2;
            j += 2;
        }

        i += 2;
    }

    // The difference along the second axis, the same thing across the rows.
    let mut i = 0;
    while i < height {
        let mut at = columns * i + 2;

        let mut j = 2;
        while j + 2 < width {
            let (upper, lower) = bounds(values[at - 2], values[at], values[at + 2]);

            if lower < upper {
                let wanted = (values[at + 2] - values[at - 2]).clamp(lower, upper);
                values[at + 1] += nudge(wanted, values[at + 1], 3, limit);
            }

            at += 2;
            j += 2;
        }

        i += 2;
    }

    // The curvature, which the four zones diagonally around this one imply, and
    // which the slopes already found constrain.
    let mut i = 2;
    while i + 2 < height {
        let mut at = columns * i + 2;

        let mut j = 2;
        while j + 2 < width {
            let here = values[at];

            // The four zones diagonally around this one, named by where they
            // sit along the slow axis and then the fast one.
            let low_low = values[at - columns2 - 2];
            let high_low = values[at + columns2 - 2];
            let low_high = values[at - columns2 + 2];
            let high_high = values[at + columns2 + 2];

            let slope_x = values[at + columns] << 1;
            let slope_y = values[at + 1] << 1;

            let upper = (((high_high - here).max(0) - slope_x - slope_y)
                .min((here - high_low).max(0) + slope_x - slope_y))
            .min(
                ((here - low_high).max(0) - slope_x + slope_y)
                    .min((low_low - here).max(0) + slope_x + slope_y),
            ) << 4;

            let lower = (((high_high - here).min(0) - slope_x - slope_y)
                .max((here - high_low).min(0) + slope_x - slope_y))
            .max(
                ((here - low_high).min(0) - slope_x + slope_y)
                    .max((low_low - here).min(0) + slope_x + slope_y),
            ) << 4;

            if lower < upper {
                let wanted = (high_high + low_low - low_high - high_low).clamp(lower, upper);
                values[at + columns + 1] += nudge(wanted, values[at + columns + 1], 6, limit);
            }

            at += 2;
            j += 2;
        }

        i += 2;
    }
}

//
// Compression
//

/// The Huffman codes the quadtree coder writes its nybbles with, and how many
/// bits each one takes.
///
/// A nybble says which of the four pixels under a node are set, and the ones
/// that come up most often — a single pixel, or a pair — get the shortest
/// codes.
const HUFFMAN_CODE: [u32; 16] = [
    0x3e, 0x00, 0x01, 0x08, 0x02, 0x09, 0x1a, 0x1b, 0x03, 0x1c, 0x0a, 0x1d, 0x0b, 0x1e, 0x3f, 0x0c,
];
const HUFFMAN_BITS: [u32; 16] = [6, 3, 3, 4, 3, 4, 5, 5, 3, 5, 4, 5, 4, 5, 6, 4];

/// Writes bits into a byte vector, most significant first.
struct Output {
    bytes: Vec<u8>,
    buffer: u32,
    free: i32,
}

impl Output {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            buffer: 0,
            free: 8,
        }
    }

    /// Writes the low `count` bits of `bits`, where `count` is at most eight.
    fn bits(&mut self, bits: u32, count: u32) {
        let mask = if count >= 32 {
            u32::MAX
        } else {
            (1 << count) - 1
        };

        self.buffer = (self.buffer << count) | (bits & mask);
        self.free -= count as i32;

        if self.free <= 0 {
            self.bytes
                .push(((self.buffer >> (-self.free)) & 0xff) as u8);
            self.free += 8;
        }
    }

    /// Writes four bits.
    fn nybble(&mut self, bits: u32) {
        self.bits(bits, 4);
    }

    /// Writes a run of nybbles, a byte at a time where they line up with one.
    fn nybbles(&mut self, values: &[u8]) {
        if values.is_empty() {
            return;
        }

        if values.len() == 1 {
            self.nybble(values[0] as u32);
            return;
        }

        let mut at = 0;

        // Only room for one nybble in the byte being filled, so it goes on its
        // own and the rest line up behind it.
        if self.free <= 4 {
            self.nybble(values[0] as u32);
            at = 1;

            if values.len() == 2 {
                self.nybble(values[1] as u32);
                return;
            }
        }

        let shift = 8 - self.free;
        let pairs = (values.len() - at) / 2;

        if self.free == 8 {
            // The nybbles fall on byte boundaries, so each pair is a byte.
            self.buffer = 0;
            for _ in 0..pairs {
                self.bytes
                    .push(((values[at] & 15) << 4) | (values[at + 1] & 15));
                at += 2;
            }
        } else {
            for _ in 0..pairs {
                let pair = (((values[at] & 15) << 4) | (values[at + 1] & 15)) as u32;
                self.buffer = (self.buffer << 8) | pair;
                at += 2;

                self.bytes.push(((self.buffer >> shift) & 0xff) as u8);
            }
        }

        // An odd nybble at the end has no partner.
        if at != values.len() {
            self.nybble(values[values.len() - 1] as u32);
        }
    }

    /// Flushes whatever is left of the byte being filled.
    fn finish(mut self) -> Vec<u8> {
        if self.free < 8 {
            self.bytes.push((self.buffer << self.free) as u8);
        }

        self.bytes
    }
}

/// Compresses one tile with HCOMPRESS.
///
/// `scale` is the factor the transform's coefficients are divided by: zero or
/// one keeps every bit, and anything larger throws away the low bits of each
/// coefficient in exchange for a smaller tile. What comes back is the tile as
/// the convention stores it, magic bytes and all.
pub(crate) fn compress(
    values: &[i64],
    rows: usize,
    columns: usize,
    scale: i64,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    if rows == 0 || columns == 0 || values.len() < rows * columns {
        return Err(format!(
            "An HCOMPRESS tile of {}x{} needs {} values, and it was given {}",
            rows,
            columns,
            rows * columns,
            values.len()
        )
        .into());
    }

    let mut a = values[..rows * columns].to_vec();

    transform(&mut a, rows, columns);
    digitize(&mut a, scale);

    Ok(encode(&mut a, rows, columns, scale))
}

/// The H-transform: the forward of [`inverse_transform`].
///
/// Each round replaces every two by two block with its sum and the three
/// differences that, with the sum, say what the block held — and then gathers
/// the sums together so that the next round can work on them alone.
fn transform(a: &mut [i64], rows: usize, columns: usize) {
    let log2n = levels(rows.max(columns));

    let mut shift = 0_u32;
    let mut mask = -2_i64;
    let mut mask2 = mask << 1;
    let mut round = 1_i64;
    let mut round2 = round << 1;
    let mut negative2 = round2 - 1;

    let (mut height, mut width) = (rows, columns);
    let mut tmp = Vec::with_capacity(rows.max(columns));

    for _ in 0..log2n {
        let odd_rows = height % 2;
        let odd_columns = width % 2;

        let mut i = 0;
        while i + 1 < height + odd_rows {
            if i >= height - odd_rows {
                break;
            }

            let mut s00 = i * columns;
            let mut s10 = s00 + columns;

            let mut j = 0;
            while j < width - odd_columns {
                let h0 = (a[s10 + 1] + a[s10] + a[s00 + 1] + a[s00]) >> shift;
                let hx = (a[s10 + 1] + a[s10] - a[s00 + 1] - a[s00]) >> shift;
                let hy = (a[s10 + 1] - a[s10] + a[s00 + 1] - a[s00]) >> shift;
                let hc = (a[s10 + 1] - a[s10] - a[s00 + 1] + a[s00]) >> shift;

                // The low bits of the sum and of the two differences are
                // dropped, which is what the inverse hands back.
                a[s10 + 1] = hc;
                a[s10] = (if hx >= 0 { hx + round } else { hx }) & mask;
                a[s00 + 1] = (if hy >= 0 { hy + round } else { hy }) & mask;
                a[s00] = (if h0 >= 0 { h0 + round2 } else { h0 + negative2 }) & mask2;

                s00 += 2;
                s10 += 2;
                j += 2;
            }

            // A row of odd length leaves a column with no partner.
            if odd_columns == 1 {
                let h0 = (a[s10] + a[s00]) << (1 - shift);
                let hx = (a[s10] - a[s00]) << (1 - shift);

                a[s10] = (if hx >= 0 { hx + round } else { hx }) & mask;
                a[s00] = (if h0 >= 0 { h0 + round2 } else { h0 + negative2 }) & mask2;
            }

            i += 2;
        }

        // A column of odd length leaves a row with no partner.
        if odd_rows == 1 {
            let mut s00 = (height - 1) * columns;

            let mut j = 0;
            while j < width - odd_columns {
                let h0 = (a[s00 + 1] + a[s00]) << (1 - shift);
                let hy = (a[s00 + 1] - a[s00]) << (1 - shift);

                a[s00 + 1] = (if hy >= 0 { hy + round } else { hy }) & mask;
                a[s00] = (if h0 >= 0 { h0 + round2 } else { h0 + negative2 }) & mask2;

                s00 += 2;
                j += 2;
            }

            if odd_columns == 1 {
                let h0 = a[s00] << (2 - shift);
                a[s00] = (if h0 >= 0 { h0 + round2 } else { h0 + negative2 }) & mask2;
            }
        }

        // Gather the coefficients of each kind together, which is what lets the
        // next round work on the sums alone.
        for i in 0..height {
            shuffle(a, columns * i, width, 1, &mut tmp);
        }
        for j in 0..width {
            shuffle(a, j, height, columns, &mut tmp);
        }

        height = height.div_ceil(2);
        width = width.div_ceil(2);

        // From the second round on, each sum is divided by two rather than
        // left as it is.
        shift = 1;
        mask = mask2;
        round = round2;
        mask2 <<= 1;
        round2 <<= 1;
        negative2 = round2 - 1;
    }
}

/// Moves the odd-numbered elements of a run to its second half, the inverse of
/// [`unshuffle`].
fn shuffle(values: &mut [i64], start: usize, n: usize, stride: usize, tmp: &mut Vec<i64>) {
    tmp.clear();

    let mut at = start + stride;
    let mut i = 1;
    while i < n {
        tmp.push(values[at]);
        at += stride * 2;
        i += 2;
    }

    // The even elements move down into the first half.
    let mut p1 = start + stride;
    let mut p2 = start + stride * 2;
    let mut i = 2;
    while i < n {
        values[p1] = values[p2];
        p1 += stride;
        p2 += stride * 2;
        i += 2;
    }

    // And the odd ones follow them.
    for value in tmp.iter() {
        values[p1] = *value;
        p1 += stride;
    }
}

/// Divides every coefficient by `scale`, which is what makes the compression
/// lossy and what smoothing later tries to undo.
fn digitize(a: &mut [i64], scale: i64) {
    if scale <= 1 {
        return;
    }

    // Rounded away from zero, so that positive and negative coefficients are
    // treated alike.
    let half = (scale + 1) / 2 - 1;

    for value in a.iter_mut() {
        *value = if *value > 0 {
            *value + half
        } else {
            *value - half
        } / scale;
    }
}

/// Writes the transformed coefficients out as the tile's bytes.
fn encode(a: &mut [i64], rows: usize, columns: usize, scale: i64) -> Vec<u8> {
    let count = rows * columns;

    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&(rows as i32).to_be_bytes());
    out.extend_from_slice(&(columns as i32).to_be_bytes());
    out.extend_from_slice(&(scale as i32).to_be_bytes());

    // The sum of everything compresses no better than it started, so it is kept
    // out of the coded planes and written whole.
    out.extend_from_slice(&a[0].to_be_bytes());
    a[0] = 0;

    // The signs are kept aside, a bit each, and the coefficients coded as
    // magnitudes.
    let mut signs: Vec<u8> = Vec::with_capacity(count.div_ceil(8));
    let mut byte = 0_u8;
    let mut free = 8;

    for value in a.iter_mut().take(count) {
        if *value != 0 {
            byte <<= 1;
            if *value < 0 {
                byte |= 1;
                *value = -*value;
            }
            free -= 1;
        }

        if free == 0 {
            signs.push(byte);
            byte = 0;
            free = 8;
        }
    }

    if free != 8 {
        signs.push(byte << free);
    }

    // How many bit planes each quadrant needs, which is how far down the coder
    // has to go there.
    let mut largest = [0_i64; 3];
    let (rows2, columns2) = (rows.div_ceil(2), columns.div_ceil(2));

    for (index, value) in a.iter().enumerate().take(count) {
        let (row, column) = (index / columns, index % columns);
        let quadrant = usize::from(column >= columns2) + usize::from(row >= rows2);

        if largest[quadrant] < *value {
            largest[quadrant] = *value;
        }
    }

    let planes: Vec<u8> = largest
        .iter()
        .map(|largest| {
            let mut left = *largest;
            let mut count = 0_u8;
            while left > 0 {
                left >>= 1;
                count += 1;
            }
            count
        })
        .collect();

    out.extend_from_slice(&planes);

    let mut bits = Output::new();

    // The four quadrants, each coded down to its own depth.
    encode_quadrant(&mut bits, a, 0, columns, rows2, columns2, planes[0]);
    encode_quadrant(
        &mut bits,
        a,
        columns2,
        columns,
        rows2,
        columns / 2,
        planes[1],
    );
    encode_quadrant(
        &mut bits,
        a,
        columns * rows2,
        columns,
        rows / 2,
        columns2,
        planes[1],
    );
    encode_quadrant(
        &mut bits,
        a,
        columns * rows2 + columns2,
        columns,
        rows / 2,
        columns / 2,
        planes[2],
    );

    // A zero nybble marks the end.
    bits.nybble(0);

    out.extend_from_slice(&bits.finish());
    out.extend_from_slice(&signs);

    out
}

/// Codes one quadrant, one bit plane at a time from the top down.
fn encode_quadrant(
    out: &mut Output,
    a: &[i64],
    offset: usize,
    stride: usize,
    rows: usize,
    columns: usize,
    planes: u8,
) {
    if planes == 0 || rows == 0 || columns == 0 {
        return;
    }

    let log2n = levels(rows.max(columns));

    // As many codes as the plane could need. A plane whose codes would not fit
    // is written as a plain bitmap instead, which is never larger.
    let limit = (rows.div_ceil(2) * columns.div_ceil(2)).div_ceil(2);

    let mut scratch = vec![0_u8; 2 * limit.max(1)];
    let mut buffer = vec![0_u8; limit.max(1)];

    for bit in (0..planes).rev() {
        let mut coder = Codes::new();
        let mut filled = 0_usize;

        // The bottom of the tree: which of each two by two block's pixels have
        // this bit set.
        one_bit(a, offset, stride, rows, columns, &mut scratch, bit);

        let (mut nx, mut ny) = (rows.div_ceil(2), columns.div_ceil(2));

        if coder.copy(&scratch[..nx * ny], &mut buffer, &mut filled, limit) {
            write_bitmap(out, a, offset, stride, rows, columns, &mut scratch, bit);
            continue;
        }

        // And each level above it, until the whole quadrant is one node.
        let mut expanded = false;
        for _ in 1..log2n {
            reduce(&mut scratch, ny, nx, ny);
            nx = nx.div_ceil(2);
            ny = ny.div_ceil(2);

            if coder.copy(&scratch[..nx * ny], &mut buffer, &mut filled, limit) {
                write_bitmap(out, a, offset, stride, rows, columns, &mut scratch, bit);
                expanded = true;
                break;
            }
        }

        if expanded {
            continue;
        }

        // The codes were built from the bottom up and are written from the top
        // down, which is the order a decoder needs them in.
        out.nybble(0xF);

        if filled == 0 {
            if coder.pending > 0 {
                out.bits(coder.buffer & ((1 << coder.pending) - 1), coder.pending);
            } else {
                // A plane with nothing in it is one node saying so.
                out.bits(HUFFMAN_CODE[0], HUFFMAN_BITS[0]);
            }
        } else {
            if coder.pending > 0 {
                out.bits(coder.buffer & ((1 << coder.pending) - 1), coder.pending);
            }
            for index in (0..filled).rev() {
                out.bits(buffer[index] as u32, 8);
            }
        }
    }
}

/// Gathers the Huffman codes of a level, a byte at a time.
struct Codes {
    buffer: u32,
    pending: u32,
}

impl Codes {
    fn new() -> Self {
        Self {
            buffer: 0,
            pending: 0,
        }
    }

    /// Adds the code of every non-zero node to `buffer`.
    ///
    /// Returns true when the codes have outgrown what the buffer holds, which
    /// says the quadtree is making this plane larger rather than smaller.
    fn copy(&mut self, nodes: &[u8], buffer: &mut [u8], filled: &mut usize, limit: usize) -> bool {
        for node in nodes {
            if *node == 0 {
                continue;
            }

            let node = *node as usize & 15;

            self.buffer |= HUFFMAN_CODE[node] << self.pending;
            self.pending += HUFFMAN_BITS[node];

            if self.pending >= 8 {
                buffer[*filled] = (self.buffer & 0xFF) as u8;
                *filled += 1;

                if *filled >= limit {
                    return true;
                }

                self.buffer >>= 8;
                self.pending -= 8;
            }
        }

        false
    }
}

/// Which of each two by two block's values have bit `bit` set, as one nybble
/// per block.
fn one_bit(
    a: &[i64],
    offset: usize,
    stride: usize,
    rows: usize,
    columns: usize,
    out: &mut [u8],
    bit: u8,
) {
    let set = |value: i64| -> u8 { ((value >> bit) & 1) as u8 };

    let mut k = 0;
    let mut i = 0;

    while i + 1 < rows {
        let mut s00 = offset + stride * i;
        let mut s10 = s00 + stride;

        let mut j = 0;
        while j + 1 < columns {
            out[k] = (set(a[s10 + 1]))
                | (set(a[s10]) << 1)
                | (set(a[s00 + 1]) << 2)
                | (set(a[s00]) << 3);

            k += 1;
            s00 += 2;
            s10 += 2;
            j += 2;
        }

        if j < columns {
            out[k] = (set(a[s10]) << 1) | (set(a[s00]) << 3);
            k += 1;
        }

        i += 2;
    }

    if i < rows {
        let mut s00 = offset + stride * i;

        let mut j = 0;
        while j + 1 < columns {
            out[k] = (set(a[s00 + 1]) << 2) | (set(a[s00]) << 3);
            k += 1;
            s00 += 2;
            j += 2;
        }

        if j < columns {
            out[k] = set(a[s00]) << 3;
        }
    }
}

/// One level up the tree: which of each two by two block of nodes is not empty.
fn reduce(nodes: &mut [u8], stride: usize, rows: usize, columns: usize) {
    let mut k = 0;
    let mut i = 0;

    while i + 1 < rows {
        let mut s00 = stride * i;
        let mut s10 = s00 + stride;

        let mut j = 0;
        while j + 1 < columns {
            nodes[k] = u8::from(nodes[s10 + 1] != 0)
                | (u8::from(nodes[s10] != 0) << 1)
                | (u8::from(nodes[s00 + 1] != 0) << 2)
                | (u8::from(nodes[s00] != 0) << 3);

            k += 1;
            s00 += 2;
            s10 += 2;
            j += 2;
        }

        if j < columns {
            nodes[k] = (u8::from(nodes[s10] != 0) << 1) | (u8::from(nodes[s00] != 0) << 3);
            k += 1;
        }

        i += 2;
    }

    if i < rows {
        let mut s00 = stride * i;

        let mut j = 0;
        while j + 1 < columns {
            nodes[k] = (u8::from(nodes[s00 + 1] != 0) << 2) | (u8::from(nodes[s00] != 0) << 3);
            k += 1;
            s00 += 2;
            j += 2;
        }

        if j < columns {
            nodes[k] = u8::from(nodes[s00] != 0) << 3;
        }
    }
}

/// Writes a bit plane as a plain bitmap, for a plane the quadtree cannot
/// shrink.
#[allow(clippy::too_many_arguments)]
fn write_bitmap(
    out: &mut Output,
    a: &[i64],
    offset: usize,
    stride: usize,
    rows: usize,
    columns: usize,
    scratch: &mut [u8],
    bit: u8,
) {
    // A zero nybble where the tree's code would be says the bitmap follows.
    out.nybble(0x0);

    one_bit(a, offset, stride, rows, columns, scratch, bit);

    let nybbles = rows.div_ceil(2) * columns.div_ceil(2);
    out.nybbles(&scratch[..nybbles]);
}

#[cfg(test)]
mod tests {
    use super::{compress, decompress, levels};

    /// The values of the "noisy" reference case: a spread of positive and
    /// negative numbers with no structure, which drives the coder down every
    /// path it has.
    fn noisy() -> Vec<i64> {
        (0..64)
            .map(|index: u32| ((index.wrapping_mul(2654435761) >> 20) % 1000) as i64 - 500)
            .collect()
    }

    /// Each of these byte streams was produced by cfitsio for the same values,
    /// so this checks the encoder against the implementation the convention was
    /// written around rather than against this crate's own decoder.
    #[test]
    fn the_encoder_writes_the_bytes_the_reference_implementation_writes() {
        let gradient: Vec<i64> = (0..64).collect();

        assert_eq!(
            compress(&gradient, 8, 8, 0).expect("an eight by eight tile"),
            vec![
                0xdd, 0x99, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x09, 0x05, 0x00, 0xf6, 0x7f, 0xef,
                0x39, 0xed, 0x7f, 0xde, 0xb3, 0xfe, 0xff, 0xbf, 0xef, 0xfb, 0xfe, 0xff, 0x83, 0xff,
                0xff, 0xfe, 0x0f, 0xff, 0xff, 0xfb, 0xfe, 0xff, 0xbf, 0xe0, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ],
            "a gradient, losslessly"
        );

        assert_eq!(
            compress(&gradient, 8, 8, 16).expect("an eight by eight tile"),
            vec![
                0xdd, 0x99, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x10,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x05, 0x01, 0x00, 0xf6, 0x7f, 0xef,
                0x39, 0xed, 0x7f, 0xdf, 0xf0, 0x7f, 0xff, 0x80, 0x00, 0x00, 0x00,
            ],
            "a gradient, at a scale of sixteen"
        );

        let odd: Vec<i64> = (0..35).map(|index: i64| (index * 37) % 101).collect();

        assert_eq!(
            compress(&odd, 5, 7, 0).expect("a five by seven tile"),
            vec![
                0xdd, 0x99, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x20, 0x07, 0x08, 0x07, 0xfb, 0x23, 0xf6,
                0xd3, 0x05, 0x74, 0x8f, 0xba, 0xd7, 0xea, 0xf3, 0xc3, 0xff, 0xbe, 0x1a, 0x08, 0xa8,
                0x0f, 0xfb, 0xde, 0xc3, 0xea, 0x03, 0xde, 0xc2, 0x2a, 0x03, 0xfe, 0xff, 0x82, 0x3c,
                0x21, 0x42, 0x3c, 0x1e, 0x81, 0xc0, 0x3d, 0x7f, 0xe0, 0x70, 0x07, 0x0f, 0xfb, 0xfe,
                0x07, 0x0f, 0xf8, 0x1c, 0x00, 0x89, 0x30, 0x64, 0x28,
            ],
            "both axes odd"
        );

        assert_eq!(
            compress(&odd, 5, 7, 4).expect("a five by seven tile"),
            vec![
                0xdd, 0x99, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x04,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc8, 0x05, 0x06, 0x05, 0xfb, 0x23, 0xf6,
                0xd3, 0x05, 0x74, 0x8f, 0xba, 0xd7, 0xea, 0xf3, 0xe1, 0xa0, 0x8a, 0x80, 0xff, 0xbd,
                0xec, 0x3e, 0xa0, 0x3d, 0xef, 0xfe, 0x08, 0xf0, 0x85, 0x08, 0xf0, 0x7a, 0x07, 0x00,
                0x70, 0x07, 0x0f, 0xfb, 0xfe, 0x07, 0x00, 0x89, 0x30, 0x64, 0x28,
            ],
            "both axes odd, at a scale of four"
        );

        assert_eq!(
            compress(&vec![7; 100], 10, 10, 0).expect("a ten by ten tile"),
            vec![
                0xdd, 0x99, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x00, 0x00, 0x00, 0x00,
            ],
            "a tile of one value, which has no differences at all"
        );
    }

    #[test]
    fn a_tile_with_no_structure_matches_the_reference_implementation() {
        // Noise drives the coder down the path where the quadtree does not pay
        // for itself and the bit plane is written out as a plain bitmap.
        assert_eq!(
            compress(&noisy(), 8, 8, 0).expect("an eight by eight tile"),
            vec![
                0xdd, 0x99, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00,
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xe0, 0x0a, 0x0b, 0x0b, 0xf0, 0xc0, 0x56,
                0x14, 0x1a, 0x9d, 0x41, 0x21, 0xc0, 0x02, 0x9e, 0x81, 0x9e, 0x1f, 0xeb, 0xc8, 0x05,
                0x83, 0x7c, 0x2f, 0xfb, 0xe5, 0x5f, 0xd8, 0x5f, 0x61, 0x7d, 0x77, 0x5f, 0x3e, 0xf0,
                0x6f, 0xa3, 0x7f, 0x89, 0x00, 0xbc, 0x0d, 0xfe, 0x30, 0x02, 0xcc, 0xdb, 0xfe, 0xff,
                0x82, 0xcb, 0xec, 0x13, 0x41, 0x03, 0x35, 0x10, 0x13, 0x41, 0x00, 0xca, 0xef, 0xfe,
                0x04, 0xd0, 0x4f, 0xfb, 0xc2, 0xff, 0xbe, 0x93, 0x03, 0x2b, 0xb0, 0x32, 0xbb, 0x03,
                0x2b, 0xb0, 0xb2, 0xfb, 0x03, 0x2b, 0xbf, 0xf8, 0x0c, 0xbe, 0xfd, 0xbf, 0x6f, 0xf9,
                0x2c, 0x00, 0x9e, 0xd1, 0xda, 0xb8, 0xf1, 0x03, 0xe0, 0x00,
            ]
        );

        assert_eq!(
            compress(&noisy(), 8, 8, 8).expect("an eight by eight tile"),
            vec![
                0xdd, 0x99, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfc, 0x07, 0x08, 0x08, 0xf0, 0xc0, 0x56,
                0x14, 0x1a, 0x9d, 0x41, 0x21, 0xc0, 0x02, 0x9e, 0x81, 0x9e, 0x1c, 0x10, 0x35, 0x3e,
                0x55, 0xfd, 0x85, 0xf6, 0x17, 0xd7, 0x74, 0x03, 0xe1, 0xe1, 0x9a, 0x89, 0xff, 0x02,
                0x68, 0x27, 0xfc, 0x16, 0x5f, 0x60, 0x9a, 0x08, 0x19, 0xa8, 0x80, 0x9a, 0x08, 0x06,
                0x57, 0x7f, 0xf0, 0x26, 0x82, 0x7d, 0x26, 0x06, 0x57, 0x60, 0x65, 0x76, 0x06, 0x57,
                0x61, 0x65, 0xf6, 0x06, 0x57, 0x7f, 0xf0, 0x59, 0x7d, 0x80, 0x9e, 0xd1, 0xda, 0xb8,
                0xf1, 0x03, 0xc0, 0x00,
            ]
        );
    }

    #[test]
    fn a_tile_of_two_columns_matches_the_reference_implementation() {
        // Three rows of two: fewer pixels than the transform has rounds, which
        // is where the edge cases of the odd dimensions all meet.
        let values: Vec<i64> = (0..6).map(|index: i64| index * 1000).collect();

        assert_eq!(
            compress(&values, 3, 2, 0).expect("a three by two tile"),
            vec![
                0xdd, 0x99, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5d, 0xc0, 0x0e, 0x0c, 0x00, 0xf3, 0xff, 0x79,
                0xf3, 0xe7, 0xfe, 0xf3, 0xe7, 0xcf, 0xfd, 0xff, 0x7f, 0xdf, 0xf7, 0xfd, 0xff, 0x7d,
                0x7d, 0x7d, 0x7d, 0x7d, 0x7f, 0xdf, 0x5f, 0xf7, 0xfd, 0xff, 0x7f, 0xde, 0xfd, 0xfb,
                0xf7, 0xef, 0xfe, 0xf7, 0xff, 0x7f, 0xdf, 0xf7, 0xfd, 0xff, 0x00, 0x00,
            ]
        );
    }

    #[test]
    fn what_the_encoder_writes_the_decoder_reads_back() {
        let cases: Vec<(&str, Vec<i64>, usize, usize)> = vec![
            ("a gradient", (0..64).collect(), 8, 8),
            ("noise", noisy(), 8, 8),
            ("one value", vec![7; 100], 10, 10),
            (
                "odd in both axes",
                (0..35).map(|i: i64| (i * 37) % 101).collect(),
                5,
                7,
            ),
            ("a single pixel", vec![42], 1, 1),
            ("one row", (0..16).collect(), 1, 16),
            ("one column", (0..16).collect(), 16, 1),
            (
                "negative values",
                (0..64).map(|i: i64| i - 32).collect(),
                8,
                8,
            ),
            (
                "a large tile",
                (0..1024)
                    .map(|i: i64| 1000 + (i % 32) * 3 + (i / 32) * 7 + (i % 7) * (i % 13))
                    .collect(),
                32,
                32,
            ),
        ];

        for (name, values, rows, columns) in cases {
            let tile = compress(&values, rows, columns, 0).expect("a tile that can be written");
            let (back, out_rows, out_columns) =
                decompress(&tile, false).expect("what this crate wrote, it can read");

            assert_eq!((out_rows, out_columns), (rows, columns), "{name}");
            assert_eq!(&back[..values.len()], values.as_slice(), "{name}");
        }
    }

    #[test]
    fn a_lossy_round_trip_stays_within_the_scale_factor() {
        let values: Vec<i64> = (0..1024)
            .map(|index: i64| 1000 + (index % 32) * 3 + (index / 32) * 7)
            .collect();

        for scale in [2_i64, 8, 32] {
            let tile = compress(&values, 32, 32, scale).expect("a tile that can be written");
            let (back, _, _) = decompress(&tile, false).expect("what this crate wrote");

            for (original, returned) in values.iter().zip(&back) {
                assert!(
                    (original - returned).abs() <= scale,
                    "at a scale of {scale}, {original} came back as {returned}"
                );
            }
        }
    }

    #[test]
    fn a_tile_of_no_pixels_is_refused() {
        let error = compress(&[], 0, 0, 0).expect_err("a tile of nothing is not a tile");

        assert!(error.to_string().contains("needs"), "got: {error}");
    }

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

    /// A lossily compressed eight by eight patch — a gradient with a bump in
    /// the middle — as cfitsio compressed it at a scale factor of sixteen.
    ///
    /// Lossy is the point: smoothing has nothing to do to a tile that was
    /// compressed losslessly, because there is no rounding to undo.
    const LOSSY_TILE: [u8; 46] = [
        0xdd, 0x99, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x10, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x88, 0x04, 0x02, 0x02, 0xf6, 0x7d, 0xeb, 0xeb, 0x34,
        0x0f, 0xdc, 0x3e, 0x1f, 0xfd, 0xe1, 0x87, 0xff, 0xbf, 0x87, 0xff, 0x00, 0x00, 0x80, 0x02,
        0x00,
    ];

    #[test]
    fn a_lossy_tile_matches_the_reference_implementation() {
        // Compressing at a scale factor throws away the low bits, so the values
        // come back in steps of the factor rather than as they went in.
        let (values, rows, columns) = decompress(&LOSSY_TILE, false).expect("a valid tile");

        assert_eq!((rows, columns), (8, 8));
        assert_eq!(
            values,
            vec![
                103, 103, 107, 107, 115, 115, 119, 119, 111, 111, 115, 115, 123, 123, 127, 127,
                115, 115, 119, 119, 127, 127, 131, 131, 123, 123, 127, 127, 135, 135, 139, 139,
                131, 131, 135, 135, 181, 149, 149, 149, 139, 139, 143, 143, 149, 149, 157, 157,
                143, 143, 147, 147, 157, 157, 161, 161, 151, 151, 155, 155, 165, 165, 169, 169,
            ]
        );
    }

    #[test]
    fn a_smoothed_tile_matches_the_reference_implementation() {
        // The same tile with SMOOTH set. The blocks of repeated values above
        // become a gradient again, and the bump in the middle survives: this is
        // exactly what cfitsio produces for this tile, value for value.
        let (values, rows, columns) = decompress(&LOSSY_TILE, true).expect("a valid tile");

        assert_eq!((rows, columns), (8, 8));
        assert_eq!(
            values,
            vec![
                103, 103, 105, 108, 113, 116, 119, 119, 111, 111, 113, 116, 121, 124, 127, 127,
                115, 115, 118, 121, 124, 127, 131, 131, 122, 122, 125, 128, 134, 137, 139, 139,
                131, 131, 133, 137, 179, 147, 149, 149, 138, 138, 140, 144, 151, 151, 157, 157,
                143, 143, 145, 149, 155, 159, 161, 161, 151, 151, 153, 157, 163, 167, 169, 169,
            ]
        );
    }

    #[test]
    fn smoothing_a_losslessly_compressed_tile_changes_nothing() {
        // A tile compressed at a scale of one gave up no bits, so there is
        // nothing for smoothing to put back and it must not move a pixel.
        let tile = [
            0xdd, 0x99, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x09, 0x05, 0x00, 0xf6, 0x7f, 0xef,
            0x39, 0xed, 0x7f, 0xde, 0xb3, 0xfe, 0xff, 0xbf, 0xef, 0xfb, 0xfe, 0xff, 0x83, 0xff,
            0xff, 0xfe, 0x0f, 0xff, 0xff, 0xfb, 0xfe, 0xff, 0xbf, 0xe0, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];

        let (plain, _, _) = decompress(&tile, false).expect("a valid tile");
        let (smoothed, _, _) = decompress(&tile, true).expect("a valid tile");

        assert_eq!(plain, smoothed);
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
