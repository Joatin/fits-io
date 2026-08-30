use std::error::Error;

/// The camera bayer pattern
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BayerPattern {
    /// Red, green on the first row; green, blue on the second.
    RGGB,
    /// Blue, green on the first row; green, red on the second.
    BGGR,
    /// Green, red on the first row; blue, green on the second.
    GRBG,
    /// Green, blue on the first row; red, green on the second.
    GBRG,
}

/// Where each colour sits inside a Bayer tile, as `(x, y)` offsets from the top
/// left of the 2x2 tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuperpixelOffsets {
    /// Where the red pixel sits in the group.
    pub red: (u32, u32),
    /// The two green samples, which are averaged together.
    pub green: [(u32, u32); 2],
    /// Where the blue pixel sits in the group.
    pub blue: (u32, u32),
}

impl BayerPattern {
    /// The position of each colour within this pattern's 2x2 tile.
    ///
    /// The pattern is named for its tile read left to right, top to bottom, so
    /// `RGGB` puts red at `(0, 0)`, green at `(1, 0)` and `(0, 1)`, and blue at
    /// `(1, 1)`.
    pub fn superpixel_offsets(&self) -> SuperpixelOffsets {
        match self {
            BayerPattern::RGGB => SuperpixelOffsets {
                red: (0, 0),
                green: [(1, 0), (0, 1)],
                blue: (1, 1),
            },
            BayerPattern::BGGR => SuperpixelOffsets {
                red: (1, 1),
                green: [(1, 0), (0, 1)],
                blue: (0, 0),
            },
            BayerPattern::GRBG => SuperpixelOffsets {
                red: (1, 0),
                green: [(0, 0), (1, 1)],
                blue: (0, 1),
            },
            BayerPattern::GBRG => SuperpixelOffsets {
                red: (0, 1),
                green: [(0, 0), (1, 1)],
                blue: (1, 0),
            },
        }
    }
}

impl From<BayerPattern> for String {
    fn from(pattern: BayerPattern) -> Self {
        match pattern {
            BayerPattern::RGGB => "RGGB".to_string(),
            BayerPattern::BGGR => "BGGR".to_string(),
            BayerPattern::GRBG => "GRBG".to_string(),
            BayerPattern::GBRG => "GBRG".to_string(),
        }
    }
}

impl TryFrom<String> for BayerPattern {
    type Error = Box<dyn Error + Send + Sync>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "rggb" => Ok(BayerPattern::RGGB),
            "bggr" => Ok(BayerPattern::BGGR),
            "grbg" => Ok(BayerPattern::GRBG),
            "gbrg" => Ok(BayerPattern::GBRG),
            _ => Err(From::from(format!("Invalid BAYERPAT value: {}", value))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BayerPattern;

    /// Reads a pattern's tile back out of its offsets, top left to bottom right.
    fn tile(pattern: BayerPattern) -> String {
        let offsets = pattern.superpixel_offsets();
        let mut tile = [' '; 4];

        let index = |(x, y): (u32, u32)| (y * 2 + x) as usize;
        tile[index(offsets.red)] = 'R';
        tile[index(offsets.green[0])] = 'G';
        tile[index(offsets.green[1])] = 'G';
        tile[index(offsets.blue)] = 'B';

        tile.iter().collect()
    }

    #[test]
    fn every_pattern_lays_out_the_tile_its_name_describes() {
        assert_eq!(tile(BayerPattern::RGGB), "RGGB");
        assert_eq!(tile(BayerPattern::BGGR), "BGGR");
        assert_eq!(tile(BayerPattern::GRBG), "GRBG");
        assert_eq!(tile(BayerPattern::GBRG), "GBRG");
    }

    #[test]
    fn every_position_in_the_tile_is_used_exactly_once() {
        for pattern in [
            BayerPattern::RGGB,
            BayerPattern::BGGR,
            BayerPattern::GRBG,
            BayerPattern::GBRG,
        ] {
            let offsets = pattern.superpixel_offsets();
            let mut positions = vec![
                offsets.red,
                offsets.green[0],
                offsets.green[1],
                offsets.blue,
            ];
            positions.sort();

            assert_eq!(
                positions,
                vec![(0, 0), (0, 1), (1, 0), (1, 1)],
                "{:?} does not cover its tile",
                pattern
            );
        }
    }
}
