//! PLIO decompression, as the tiled image convention uses it.
//!
//! PLIO comes from IRAF, where it stores the integer masks that mark bad pixels
//! or divide an image into regions. Such a mask is mostly long runs of one
//! value, so it is coded as a list of instructions — output this many zeros,
//! output this many of the current value, change the current value — rather than
//! as pixels.
//!
//! Every instruction is one 16-bit word: a sign bit that is not used, a three
//! bit opcode, and twelve bits of data. A run or a change of value too large for
//! twelve bits is written as several instructions.

use std::error::Error;

/// What an instruction tells the decoder to do.
///
/// The names are the convention's own.
#[derive(Debug, Clone, Copy)]
enum Instruction {
    /// `ZN`: output `n` zeros.
    Zero,
    /// `SH`: set the high value outright, taking its upper bits from the word
    /// that follows.
    SetHigh,
    /// `IH`: raise the high value by `n`, without outputting anything.
    IncrementHigh,
    /// `DH`: lower the high value by `n`, without outputting anything.
    DecrementHigh,
    /// `HN`: output `n` of the high value.
    High,
    /// `PN`: output `n - 1` zeros and then one high value.
    ZerosThenHigh,
    /// `IS`: raise the high value by `n`, then output one of it.
    IncrementAndStep,
    /// `DS`: lower the high value by `n`, then output one of it.
    DecrementAndStep,
}

impl Instruction {
    fn from_opcode(opcode: u16) -> Option<Self> {
        Some(match opcode {
            0 => Instruction::Zero,
            1 => Instruction::SetHigh,
            2 => Instruction::IncrementHigh,
            3 => Instruction::DecrementHigh,
            4 => Instruction::High,
            5 => Instruction::ZerosThenHigh,
            6 => Instruction::IncrementAndStep,
            7 => Instruction::DecrementAndStep,
            _ => return None,
        })
    }
}

/// Where the list records how long its own header is.
const HEADER_LENGTH: usize = 1;

/// The header length every writer in practice uses, for a list whose own header
/// field is not believable.
const USUAL_HEADER_LENGTH: usize = 7;

/// Decompresses one PLIO-coded tile into `count` values.
pub(crate) fn decompress(
    words: &[u16],
    count: usize,
) -> Result<Vec<i64>, Box<dyn Error + Send + Sync>> {
    if count == 0 {
        return Ok(Vec::new());
    }

    // The list opens with a header giving, among other things, its own length.
    // A length that does not fit the list is not one to seek past.
    let header = words
        .get(HEADER_LENGTH)
        .map(|length| *length as usize)
        .filter(|length| *length < words.len())
        .unwrap_or(USUAL_HEADER_LENGTH);

    let mut values = Vec::with_capacity(count);

    // A mask is a boolean one unless an instruction says otherwise, so the value
    // it marks pixels with starts at one.
    let mut high = 1_i64;
    let mut at = header;

    while values.len() < count {
        let Some(word) = words.get(at) else {
            return Err(format!(
                "A PLIO compressed tile ran out after {} of {} values",
                values.len(),
                count
            )
            .into());
        };
        at += 1;

        let data = (word & 0x0FFF) as i64;
        let opcode = (word >> 12) & 0x7;

        let instruction = Instruction::from_opcode(opcode)
            .ok_or_else(|| format!("A PLIO tile holds an unknown instruction: {}", opcode))?;

        // Never write past what was asked for, however long the run says it is.
        let remaining = count - values.len();

        match instruction {
            Instruction::Zero => {
                values.resize(values.len() + (data as usize).min(remaining), 0);
            }
            Instruction::High => {
                values.resize(values.len() + (data as usize).min(remaining), high);
            }
            Instruction::ZerosThenHigh => {
                let zeros = data.max(1) as usize - 1;
                values.resize(values.len() + zeros.min(remaining), 0);

                if values.len() < count {
                    values.push(high);
                }
            }

            // The high value's upper bits come from the word after this one,
            // which is how a value wider than twelve bits is written.
            Instruction::SetHigh => {
                let upper = words.get(at).copied().ok_or(
                    "A PLIO tile ends part way through an instruction that sets its high value",
                )?;
                at += 1;

                high = (((upper & 0x7FFF) as i64) << 12) | data;
            }

            Instruction::IncrementHigh => high += data,
            Instruction::DecrementHigh => high -= data,
            Instruction::IncrementAndStep => {
                high += data;
                values.push(high);
            }
            Instruction::DecrementAndStep => {
                high -= data;
                values.push(high);
            }
        }
    }

    values.truncate(count);

    Ok(values)
}

/// The largest run, or change of value, one instruction can carry.
const MAX_DATA: i64 = 4095;

/// The largest value the list can name at all.
///
/// Setting the value outright spends twelve bits on one word and fifteen on the
/// next, and there is nowhere to put anything above that.
const MAX_VALUE: i64 = (1 << 27) - 1;

/// Compresses a mask into a PLIO line list.
///
/// The instructions are produced in the order and form the reference
/// implementation produces them, so a list written here is the list `fpack`
/// would have written for the same mask.
///
/// # Errors
///
/// Returns an error for a value the list cannot name. PLIO stores masks, and a
/// mask value is a small non-negative number; a negative one is stored as zero,
/// which is what the reference implementation does with it, but a value beyond
/// twenty-seven bits has nowhere to go and is refused rather than truncated.
pub(crate) fn compress(values: &[i64]) -> Result<Vec<u16>, Box<dyn Error + Send + Sync>> {
    // Word 0 is unused, word 1 the header's own length, word 2 a magic number,
    // words 3 and 4 the length of the whole list, and words 5 and 6 unused.
    let mut words: Vec<u16> = vec![0, USUAL_HEADER_LENGTH as u16, 0xff9c, 0, 0, 0, 0];

    if values.is_empty() {
        return Ok(words);
    }

    if let Some(value) = values.iter().find(|value| **value > MAX_VALUE) {
        return Err(format!(
            "A PLIO mask holds values up to {}, and this one holds {}",
            MAX_VALUE, value
        )
        .into());
    }

    // A negative value is not a mask value; the reference implementation reads
    // one as zero, and a list that said otherwise would not decode the same way
    // there.
    let at = |index: usize| values[index].max(0);

    let last = values.len() - 1;

    // The value the run being gathered is made of, where that run began, and
    // where the zeros before it began.
    let mut current = at(0);
    let mut run_start = 0_usize;
    let mut zeros_start = 0_usize;

    // The value the list is currently set to output, which starts at one
    // because a plain boolean mask never has to set it at all.
    let mut high = 1_i64;

    let mut next = current;

    for index in 0..values.len() {
        if index < last {
            next = at(index + 1);

            // Still inside the same run.
            if next == current {
                continue;
            }

            // A run of zeros is not written as a run; it is the gap before the
            // next run of something.
            if current == 0 {
                current = next;
                run_start = index + 1;
                continue;
            }
        } else if current == 0 {
            // The mask ends in zeros, which are a gap with no run after them.
            run_start = values.len();
        }

        let mut run = index as i64 - run_start as i64 + 1;
        let mut gap = run_start as i64 - zeros_start as i64;

        let mut done = false;

        if current > 0 {
            let change = current - high;

            if change != 0 {
                high = current;

                if change.abs() > MAX_DATA {
                    // Too big a change to describe as one: the value is set
                    // outright, across two words.
                    words.push(((current & 0xFFF) + 0x1000) as u16);
                    words.push((current >> 12) as u16);
                } else {
                    if change < 0 {
                        words.push((-change + 0x3000) as u16);
                    } else {
                        words.push((change + 0x2000) as u16);
                    }

                    // A single pixel right after the change needs no run of its
                    // own: the instruction that changes the value can output it
                    // as well.
                    if run == 1 && gap == 0 {
                        let last = words.len() - 1;
                        words[last] |= 0x4000;
                        done = true;
                    }
                }
            }
        }

        if !done && gap > 0 {
            while gap > 0 {
                words.push(gap.min(MAX_DATA) as u16);
                gap -= MAX_DATA;
            }

            // A single pixel after a gap likewise rides along with the gap.
            if run == 1 && current > 0 {
                let last = words.len() - 1;
                words[last] = words[last].wrapping_add(0x5001);
                done = true;
            }
        }

        if !done {
            while run > 0 {
                words.push((run.min(MAX_DATA) + 0x4000) as u16);
                run -= MAX_DATA;
            }
        }

        run_start = index + 1;
        zeros_start = run_start;
        current = next;
    }

    // The list records how long it is, in two words.
    let length = words.len();
    words[3] = (length % 32768) as u16;
    words[4] = (length / 32768) as u16;

    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::{compress, decompress};

    /// A list with the seven word header these all carry.
    fn list(instructions: &[u16]) -> Vec<u16> {
        let mut words = vec![0, 7, 0xff9c, (7 + instructions.len()) as u16, 0, 0, 0];
        words.extend_from_slice(instructions);
        words
    }

    /// Every list here was produced by cfitsio's own encoder, so these check
    /// this one against the implementation the convention was written around
    /// rather than against this crate's decoder.
    #[test]
    fn the_encoder_writes_the_list_the_reference_implementation_writes() {
        let cases: Vec<(&str, Vec<i64>, Vec<u16>)> = vec![
            (
                "alternating runs",
                vec![0, 0, 0, 1, 1, 1, 0, 0],
                vec![
                    0x0000, 0x0007, 0xff9c, 0x000a, 0x0000, 0x0000, 0x0000, 0x0003, 0x4003, 0x0002,
                ],
            ),
            (
                "a value that steps up and down",
                vec![0, 1, 2, 3, 0, 0, 5, 5],
                vec![
                    0x0000, 0x0007, 0xff9c, 0x000d, 0x0000, 0x0000, 0x0000, 0x5002, 0x6001, 0x6001,
                    0x2002, 0x0002, 0x4002,
                ],
            ),
            (
                "nothing but zeros",
                vec![0; 8],
                vec![
                    0x0000, 0x0007, 0xff9c, 0x0008, 0x0000, 0x0000, 0x0000, 0x0008,
                ],
            ),
            (
                "nothing but ones",
                vec![1; 8],
                vec![
                    0x0000, 0x0007, 0xff9c, 0x0008, 0x0000, 0x0000, 0x0000, 0x4008,
                ],
            ),
            (
                "a value too wide for one instruction",
                vec![1000000, 1000000, 0, 3, 3, 3, 0, 7],
                vec![
                    0x0000, 0x0007, 0xff9c, 0x0010, 0x0000, 0x0000, 0x0000, 0x1240, 0x00f4, 0x4002,
                    0x1003, 0x0000, 0x0001, 0x4003, 0x2004, 0x5002,
                ],
            ),
            (
                "single pixels between gaps",
                vec![0, 0, 7, 0, 0, 9, 0, 0],
                vec![
                    0x0000, 0x0007, 0xff9c, 0x000c, 0x0000, 0x0000, 0x0000, 0x2006, 0x5003, 0x2002,
                    0x5003, 0x0002,
                ],
            ),
            (
                "a pixel at each end",
                vec![5, 0, 0, 0, 0, 0, 0, 5],
                vec![
                    0x0000, 0x0007, 0xff9c, 0x0009, 0x0000, 0x0000, 0x0000, 0x6004, 0x5007,
                ],
            ),
        ];

        for (name, values, expected) in cases {
            assert_eq!(
                compress(&values).expect("a mask that can be written"),
                expected,
                "{name}"
            );
        }
    }

    #[test]
    fn a_run_longer_than_one_instruction_is_written_as_several() {
        // Forty-five hundred of one value, where an instruction carries at most
        // 4095 of them.
        let mut values = vec![4_i64; 4500];
        values.resize(5000, 0);

        assert_eq!(
            compress(&values).expect("a mask that can be written"),
            vec![
                0x0000, 0x0007, 0xff9c, 0x000b, 0x0000, 0x0000, 0x0000, 0x2003, 0x4fff, 0x4195,
                0x01f4
            ]
        );
    }

    #[test]
    fn what_the_encoder_writes_the_decoder_reads_back() {
        let cases: Vec<Vec<i64>> = vec![
            vec![0, 0, 0, 1, 1, 1, 0, 0],
            vec![0, 1, 2, 3, 0, 0, 5, 5],
            vec![0; 40],
            vec![1; 40],
            vec![1000000, 1000000, 0, 3, 3, 3, 0, 7],
            (0..200).map(|index| (index % 5) as i64).collect(),
            (0..5000)
                .map(|index| if index < 4500 { 4 } else { 0 })
                .collect(),
            vec![5, 0, 0, 0, 0, 0, 0, 5],
            vec![7],
        ];

        for values in cases {
            let list = compress(&values).expect("a mask that can be written");
            let back = decompress(&list, values.len()).expect("what this crate wrote");

            assert_eq!(back, values);
        }
    }

    #[test]
    fn a_value_the_list_cannot_name_is_refused() {
        // Beyond twenty-seven bits there is nowhere in an instruction to put
        // the value, and truncating it would write a different mask.
        let error = compress(&[1 << 28]).expect_err("that value does not fit a line list");

        assert!(
            error.to_string().contains("holds values up to"),
            "got: {error}"
        );
    }

    /// These four were produced by cfitsio, through astropy, so they check this
    /// decoder against the implementation the convention was written around.
    #[test]
    fn a_run_of_zeros_matches_the_reference_implementation() {
        assert_eq!(decompress(&list(&[0x0008]), 8).unwrap(), vec![0; 8]);
    }

    #[test]
    fn a_run_of_the_high_value_matches_the_reference_implementation() {
        assert_eq!(decompress(&list(&[0x4008]), 8).unwrap(), vec![1; 8]);
    }

    #[test]
    fn alternating_runs_match_the_reference_implementation() {
        assert_eq!(
            decompress(&list(&[0x0003, 0x4003, 0x0002]), 8).unwrap(),
            vec![0, 0, 0, 1, 1, 1, 0, 0]
        );
    }

    #[test]
    fn changing_the_high_value_matches_the_reference_implementation() {
        // Exercises PN, IS twice, IH, ZN and HN in one line.
        assert_eq!(
            decompress(&list(&[0x5002, 0x6001, 0x6001, 0x2002, 0x0002, 0x4002]), 8).unwrap(),
            vec![0, 1, 2, 3, 0, 0, 5, 5]
        );
    }

    #[test]
    fn a_high_value_wider_than_twelve_bits_is_set_from_two_words() {
        // SH takes its low twelve bits from the instruction and the rest from
        // the word after it, which is the only way to reach a large mask value.
        let value = 1_000_000_i64;
        let low = (value & 0x0FFF) as u16;
        let high = (value >> 12) as u16;

        assert_eq!(
            decompress(&list(&[0x1000 | low, high, 0x4002]), 2).unwrap(),
            vec![value, value]
        );
    }

    #[test]
    fn the_high_value_can_be_decremented() {
        // DH and DS are the counterparts of IH and IS.
        assert_eq!(
            decompress(&list(&[0x2064, 0x7001, 0x3002, 0x4001]), 2).unwrap(),
            vec![100, 98]
        );
    }

    #[test]
    fn a_run_longer_than_one_instruction_can_hold_matches_the_reference() {
        // Twelve bits of data reach 4095, so 9000 zeros take three
        // instructions. cfitsio wrote these; the point is that the runs add up.
        let words = [
            0x0000, 0x0007, 0xff9c, 0x000a, 0x0000, 0x0000, 0x0000, 0x0fff, 0x0fff, 0x032a,
        ];

        assert_eq!(decompress(&words, 9000).unwrap(), vec![0; 9000]);
    }

    #[test]
    fn a_run_of_each_value_matches_the_reference() {
        // Five thousand zeros and then five thousand ones, each spanning more
        // than one instruction.
        let words = [
            0x0000, 0x0007, 0xff9c, 0x000b, 0x0000, 0x0000, 0x0000, 0x0fff, 0x0389, 0x4fff, 0x4389,
        ];

        let mut expected = vec![0_i64; 5000];
        expected.extend(std::iter::repeat_n(1_i64, 5000));

        assert_eq!(decompress(&words, 10000).unwrap(), expected);
    }

    #[test]
    fn values_across_the_masks_full_depth_match_the_reference() {
        // PLIO reaches about 24 bits, and getting there needs SH and several
        // increments. 16777215 is 2^24 - 1.
        let words = [
            0x0000, 0x0007, 0xff9c, 0x0010, 0x0000, 0x0000, 0x0000, 0x16a0, 0x0018, 0x5002, 0x1fff,
            0x0fff, 0x5002, 0x1005, 0x0000, 0x4001,
        ];

        assert_eq!(
            decompress(&words, 5).unwrap(),
            vec![0, 100_000, 0, 16_777_215, 5]
        );
    }

    #[test]
    fn a_run_longer_than_the_tile_stops_at_its_end() {
        // A malformed list must not write past the tile it is filling.
        assert_eq!(decompress(&list(&[0x0FFF]), 4).unwrap(), vec![0; 4]);
    }

    #[test]
    fn a_list_that_ends_early_is_an_error() {
        let error = decompress(&list(&[0x0002]), 8).expect_err("the list ends early");

        assert!(error.to_string().contains("ran out"), "got: {error}");
    }
}
