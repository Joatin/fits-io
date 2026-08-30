//! The FITS checksum convention.
//!
//! An HDU can carry two cards that let a reader tell whether it survived being
//! copied around: DATASUM over the data section alone, and CHECKSUM over the
//! whole HDU. The scheme is arranged so that summing a complete, undamaged HDU —
//! its CHECKSUM card included — gives all ones, which is what
//! [`verify`] checks.

/// The 32-bit one's complement sum of `bytes`, continuing from `initial`.
///
/// Bytes are taken four at a time as big-endian words, and the carry out of the
/// top is added back in at the bottom. A trailing partial word is padded with
/// zeros, which only arises for input that is not a whole number of FITS blocks.
pub(crate) fn sum32(bytes: &[u8], initial: u32) -> u32 {
    let mut sum = initial as u64;

    let (words, remainder) = bytes.as_chunks::<4>();
    for word in words {
        sum += u32::from_be_bytes(*word) as u64;
    }

    if !remainder.is_empty() {
        let mut word = [0_u8; 4];
        word[..remainder.len()].copy_from_slice(remainder);
        sum += u32::from_be_bytes(word) as u64;
    }

    // Fold the carries back in, twice: folding once can itself carry.
    while sum > u32::MAX as u64 {
        sum = (sum & u32::MAX as u64) + (sum >> 32);
    }

    sum as u32
}

/// The value a CHECKSUM card should hold, given the sum over an HDU whose
/// CHECKSUM card is present but blank.
pub(crate) fn complement(sum: u32) -> u32 {
    !sum
}

/// Whether an HDU's bytes carry a correct checksum.
///
/// A complete HDU, CHECKSUM card and all, sums to all ones by construction, so
/// this is how a reader tells an intact HDU from a damaged one. `hdu` must be
/// one whole HDU: its header blocks followed by its padded data section.
pub fn verify(hdu: &[u8]) -> bool {
    sum32(hdu, 0) == u32::MAX
}

/// The 16 characters a CHECKSUM card's value is written as.
///
/// The convention encodes the complemented sum as printable ASCII rather than a
/// number, so that the card is the same length whatever the value, and so that
/// the card contributes to the very sum it records. The encoding spreads each
/// byte of the value over four characters and avoids the punctuation between
/// `9` and `A`, which would otherwise make the value hard to read back.
pub(crate) fn encode(value: u32) -> String {
    // The characters between the digits and the letters, which the convention
    // steps around so that a checksum stays alphanumeric.
    const EXCLUDED: [u8; 13] = *b":;<=>?@[\\]^_`";
    const OFFSET: u8 = b'0';

    let mut ascii = [0_u8; 16];

    for (index, byte) in value.to_be_bytes().iter().enumerate() {
        let quotient = byte / 4 + OFFSET;
        let remainder = byte % 4;

        let mut characters = [quotient; 4];
        characters[0] = characters[0].wrapping_add(remainder);

        // Shifting a character up and its partner down keeps the four-character
        // group's total, and so the checksum, unchanged.
        loop {
            let mut adjusted = false;

            for excluded in EXCLUDED {
                for pair in 0..2 {
                    let (first, second) = (pair * 2, pair * 2 + 1);
                    if characters[first] == excluded || characters[second] == excluded {
                        characters[first] = characters[first].wrapping_add(1);
                        characters[second] = characters[second].wrapping_sub(1);
                        adjusted = true;
                    }
                }
            }

            if !adjusted {
                break;
            }
        }

        for (group, character) in characters.iter().enumerate() {
            ascii[4 * group + index] = *character;
        }
    }

    // The convention rotates the result one place to the right.
    let mut rotated = String::with_capacity(16);
    rotated.push(ascii[15] as char);
    rotated.extend(ascii[..15].iter().map(|byte| *byte as char));

    rotated
}

#[cfg(test)]
mod tests {
    use super::{complement, encode, sum32, verify};

    #[test]
    fn a_sum_folds_its_carries_back_in() {
        // Two words that overflow: the carry out of the top comes back at the
        // bottom rather than being dropped.
        let bytes = [0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x02];

        assert_eq!(sum32(&bytes, 0), 2);
    }

    #[test]
    fn a_partial_trailing_word_is_padded_rather_than_dropped() {
        assert_eq!(sum32(&[0x00, 0x00, 0x01], 0), 0x0000_0100);
    }

    #[test]
    fn an_encoded_checksum_is_sixteen_printable_characters() {
        for value in [0_u32, 1, 0xFFFF_FFFF, 0x1234_5678, 0xDEAD_BEEF] {
            let encoded = encode(value);

            assert_eq!(encoded.len(), 16, "{value:#x} encoded as {encoded:?}");
            assert!(
                encoded.chars().all(|c| c.is_ascii_alphanumeric()),
                "{value:#x} encoded as {encoded:?}, which is not alphanumeric"
            );
        }
    }

    #[test]
    fn an_encoded_checksum_sums_to_the_value_it_stands_for() {
        // This is the whole point of the encoding: the sixteen characters, taken
        // as four big-endian words, add back up to the value they encode -- plus
        // the ASCII-zero offset carried by each of the sixteen characters, which
        // the rest of the card accounts for. If this did not hold, writing the
        // card would change the very checksum it records.
        const OFFSETS: u32 = 0xC0C0_C0C0;

        for value in [0_u32, 1, 0x1234_5678, 0xDEAD_BEEF, 0xFFFF_FFFF] {
            // Undo the rotation the encoder applied.
            let encoded = encode(value);
            let bytes = encoded.as_bytes();
            let mut unrotated = Vec::with_capacity(16);
            unrotated.extend_from_slice(&bytes[1..]);
            unrotated.push(bytes[0]);

            assert_eq!(
                sum32(&unrotated, 0),
                sum32(&OFFSETS.to_be_bytes(), value),
                "{value:#x} encoded as {encoded:?}"
            );
        }
    }

    #[test]
    fn a_complemented_sum_makes_the_whole_thing_all_ones() {
        let data = b"some bytes to check over, padded out";
        let sum = sum32(data, 0);

        // Adding the complement is what the CHECKSUM card does, and the result
        // is what `verify` looks for.
        let total = sum32(&complement(sum).to_be_bytes(), sum);

        assert_eq!(total, u32::MAX);
    }

    #[test]
    fn verify_rejects_altered_bytes() {
        let mut hdu = vec![0_u8; 16];
        let sum = sum32(&hdu, 0);
        hdu.extend_from_slice(&complement(sum).to_be_bytes());

        assert!(verify(&hdu));

        hdu[3] ^= 0x01;
        assert!(!verify(&hdu));
    }
}
