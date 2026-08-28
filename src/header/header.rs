use crate::header::card::Card;
use crate::header::extension_type::ExtensionType;
use crate::header::value::Value;
use crate::header::{BayerPattern, Bitpix, ImageType, TableColumnFormat};
use crate::util::ReadSeek;
use chrono::{DateTime, Utc};
use std::error::Error;
use std::fmt::Formatter;
use std::io::Read;
use std::{fmt, vec};

const CARD_NUM_BYTES: usize = 80;

/// FITS files are laid out in blocks of this many bytes; headers and data
/// sections are both padded up to a whole number of them.
const BLOCK_NUM_BYTES: usize = 2880;

#[derive(Clone, Default)]
pub struct Header {
    cards: Vec<Card>,
}

impl Header {
    pub(crate) fn bytes_len(&self) -> usize {
        let num_bytes = self.cards.len() * CARD_NUM_BYTES;
        let num_off_bytes = BLOCK_NUM_BYTES - (num_bytes % BLOCK_NUM_BYTES);
        if num_off_bytes == BLOCK_NUM_BYTES {
            num_bytes
        } else {
            num_bytes + num_off_bytes
        }
    }

    pub fn author(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Author { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    /// The BITPIX card.
    ///
    /// BITPIX is mandatory, so this returns `Some` for any header that passed
    /// [`Header::validate_primary`] or [`Header::validate_extension`]. It returns
    /// `None` for a header that was built by hand and left incomplete.
    pub fn bitpix(&self) -> Option<Bitpix> {
        self.cards.iter().find_map(|card| {
            if let Card::Bitpix { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn blank(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::Blank { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn blocked(&self) -> Option<bool> {
        self.cards.iter().find_map(|card| {
            if let Card::Blocked { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn bscale(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::BScale { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// BSCALE, defaulting to 1.0 when the card is absent.
    ///
    /// BSCALE is optional; the FITS standard defines its default as 1.0, so a
    /// missing card means unscaled data rather than unknown data.
    pub fn bscale_or_default(&self) -> f64 {
        self.bscale().unwrap_or(1.0)
    }

    pub fn bunit(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::BUnit { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    pub fn bzero(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::BZero { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// BZERO, defaulting to 0.0 when the card is absent.
    ///
    /// BZERO is optional; the FITS standard defines its default as 0.0, so a
    /// missing card means unshifted data rather than unknown data.
    pub fn bzero_or_default(&self) -> f64 {
        self.bzero().unwrap_or(0.0)
    }

    pub fn data_max(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::DataMax { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn data_min(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::DataMin { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn date(&self) -> Option<&DateTime<Utc>> {
        self.cards.iter().find_map(|card| {
            if let Card::Date { value, .. } = card {
                Some(value)
            } else {
                None
            }
        })
    }

    pub fn date_observed(&self) -> Option<&DateTime<Utc>> {
        self.cards.iter().find_map(|card| {
            if let Card::DateObserved { value, .. } = card {
                Some(value)
            } else {
                None
            }
        })
    }

    pub fn epoch(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::Epoch { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn equinox(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::Equinox { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn extend(&self) -> Option<bool> {
        self.cards.iter().find_map(|card| {
            if let Card::Extend { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn extension_level(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::ExtensionLevel { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn extension_name(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::ExtensionName { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    pub fn extension_version(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::ExtensionVersion { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn group_count(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::GroupCount { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn groups(&self) -> Option<bool> {
        self.cards.iter().find_map(|card| {
            if let Card::Groups { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn instrument(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Instrument { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    /// The NAXIS card, i.e. the number of data axes.
    ///
    /// NAXIS is mandatory, so this returns `Some` for any header that passed
    /// [`Header::validate_primary`] or [`Header::validate_extension`]. It returns
    /// `None` for a header that was built by hand and left incomplete.
    pub fn naxis(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::NAxis { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn object(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Object { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    pub fn observer(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Observer { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    pub fn origin(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Origin { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    pub fn pcount(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::ParameterCount { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn reference(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Reference { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    pub fn simple(&self) -> Option<bool> {
        self.cards.iter().find_map(|card| {
            if let Card::Simple { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn telescope(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Telescope { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    pub fn table_fields(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::TableFields { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn table_heap(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::TableHeap { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn extension(&self) -> Option<ExtensionType> {
        self.cards.iter().find_map(|card| {
            if let Card::Xtension { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn focal_length(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::FocalLength { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn exposure_time(&self) -> Option<std::time::Duration> {
        self.cards.iter().find_map(|card| {
            if let Card::ExposureTime { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn ccd_temperature(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::CCDTemperature { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn bayer_pattern(&self) -> Option<BayerPattern> {
        self.cards.iter().find_map(|card| {
            if let Card::BayerPattern { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn creator(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Creator { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    pub fn subframe_x_position_in_binned_pixels(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::SubframeXPositionInBinnedPixels { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn subframe_y_position_in_binned_pixels(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::SubframeYPositionInBinnedPixels { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn binned_pixels_x(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::BinnedPixelsX { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn binned_pixels_y(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::BinnedPixelsY { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn ccd_binned_pixels_x(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::CCDBinnedPixelsX { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn ccd_binned_pixels_y(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::CCDBinnedPixelsY { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn pixel_size_x_with_binning_in_microns(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::PixelSizeXWithBinningInMicrons { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn pixel_size_y_with_binning_in_microns(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::PixelSizeYWithBinningInMicrons { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn image_type(&self) -> Option<&ImageType> {
        self.cards.iter().find_map(|card| {
            if let Card::ImageType { value, .. } = card {
                Some(value)
            } else {
                None
            }
        })
    }

    pub fn exposure(&self) -> Option<std::time::Duration> {
        self.cards.iter().find_map(|card| {
            if let Card::Exposure { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn ra(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::Ra { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn dec(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::Dec { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn guide_cam(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::GuideCam { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    pub fn focus_position(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::FocusPosition { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn site_longitude(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::SiteLongitude { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn site_latitude(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::SiteLatitude { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn image_width(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::ImageWidth { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn image_height(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::ImageHeight { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    pub fn coordinate_delta(&self, index: usize) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::CoordinateDeltaN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(*value);
            };
            None
        })
    }

    pub fn coordinate_rotation(&self, index: usize) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::CoordinateRotationN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(*value);
            };
            None
        })
    }

    pub fn coordinate_reference_pixel(&self, index: usize) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::CoordinateReferencePixelN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(*value);
            };
            None
        })
    }

    pub fn coordinate_value_at_pixel(&self, index: usize) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::CoordinateValueAtPixelN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(*value);
            };
            None
        })
    }

    pub fn coordinate_axis_name(&self, index: usize) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::CoordinateAxisNameN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(value.as_str());
            };
            None
        })
    }

    pub fn naxis_n(&self, index: usize) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::NAxisN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(*value);
            };
            None
        })
    }

    pub fn parameter_scaling_factor(&self, index: usize) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::ParameterScalingFactorN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(*value);
            };
            None
        })
    }

    pub fn parameter_type(&self, index: usize) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::ParameterTypeN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(value.as_str());
            };
            None
        })
    }

    pub fn parameter_scaling_zero_point(&self, index: usize) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::ParameterScalingZeroPointN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(*value);
            };
            None
        })
    }

    pub fn table_column(&self, index: usize) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::TableColumnN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(*value);
            };
            None
        })
    }

    pub fn table_dimensions(&self, index: usize) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::TableDimensionsN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(value.as_str());
            };
            None
        })
    }

    pub fn table_display_format(&self, index: usize) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::TableDisplayFormatN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(value.as_str());
            };
            None
        })
    }

    pub fn table_null_value(&self, index: usize) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::TableNullValueN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(value.as_str());
            };
            None
        })
    }

    pub fn table_scaling_factor(&self, index: usize) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::TableScalingFactorN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(*value);
            };
            None
        })
    }

    pub fn table_column_type(&self, index: usize) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::TableTypeN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(value.as_str());
            };
            None
        })
    }

    pub fn table_column_format(&self, index: usize) -> Option<TableColumnFormat> {
        self.cards.iter().find_map(|card| {
            if let Card::TableFormatN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(*value);
            };
            None
        })
    }

    pub fn table_unit(&self, index: usize) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::TableUnitN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(value.as_str());
            };
            None
        })
    }

    pub fn table_scaling_zero_point(&self, index: usize) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::TableScalingZeroPointN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(*value);
            };
            None
        })
    }

    pub(crate) fn data_block_len(&self) -> usize {
        let data_size = self.data_bytes_len();

        let num_off_bytes = BLOCK_NUM_BYTES - (data_size % BLOCK_NUM_BYTES);
        if num_off_bytes == BLOCK_NUM_BYTES {
            data_size
        } else {
            data_size + num_off_bytes
        }
    }

    /// Size of this HDU's data section in bytes, excluding block padding.
    ///
    /// Returns 0 for a header that declares no data, and also for an incomplete
    /// header — a header missing BITPIX, NAXIS or one of its NAXISn cards cannot
    /// describe a data section. [`Header::validate_primary`] and
    /// [`Header::validate_extension`] reject such headers up front, so this
    /// fallback is only reachable for hand-built headers.
    pub(crate) fn data_bytes_len(&self) -> usize {
        let (Some(bitpix), Some(number_of_axis)) = (self.bitpix(), self.naxis()) else {
            return 0;
        };

        if number_of_axis <= 0 {
            return 0;
        }

        let mut bytes = bitpix.byte_size();
        for axis in 0..number_of_axis {
            // NAXISn is untrusted input: a negative or absurd length must not
            // overflow the running product.
            let Some(length) = self.naxis_n(axis as usize) else {
                return 0;
            };
            let Ok(length) = usize::try_from(length) else {
                return 0;
            };
            let Some(product) = bytes.checked_mul(length) else {
                return 0;
            };
            bytes = product;
        }
        bytes
    }

    pub fn raw_card(&self, key: &str) -> Vec<Value> {
        self.cards
            .iter()
            .filter_map(|card| {
                if key == card.key() {
                    Some(Value::from(card))
                } else {
                    None
                }
            })
            .collect()
    }

    pub(crate) fn from_reader(
        reader: &mut Box<dyn ReadSeek>,
    ) -> Result<Option<Self>, Box<dyn Error + Send + Sync>> {
        let cards = Self::read_all_cards(reader)?;

        if let Some(Card::End) = cards.last() {
            Ok(Some(Self { cards }))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn validate_primary(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.simple().is_none() {
            return Err("This is not a valid fits file. Card SIMPLE is missing".into());
        }
        if let Some(false) = self.simple() {
            return Err(
                "This is not a valid fits file. It must contain card simple with value true".into(),
            );
        }
        self.validate_structure("fits file")?;

        Ok(())
    }

    pub(crate) fn validate_extension(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.extension().is_none() {
            return Err("This is not a valid fits extension. Card XTENSION is missing".into());
        }
        self.validate_structure("fits extension")?;

        Ok(())
    }

    /// Checks the structural cards every HDU must carry: BITPIX, NAXIS and one
    /// NAXISn per axis.
    ///
    /// Callers rely on this: once a header has been validated, [`Header::bitpix`],
    /// [`Header::naxis`] and [`Header::naxis_n`] are known to return `Some`, and
    /// [`Header::data_bytes_len`] is known to describe the real data section.
    fn validate_structure(&self, kind: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.bitpix().is_none() {
            return Err(format!("This is not a valid {}. Card BITPIX is missing", kind).into());
        }

        let Some(number_of_axis) = self.naxis() else {
            return Err(format!("This is not a valid {}. Card NAXIS is missing", kind).into());
        };

        if number_of_axis < 0 {
            return Err(format!(
                "This is not a valid {}. Card NAXIS must not be negative, but was {}",
                kind, number_of_axis
            )
            .into());
        }

        for axis in 0..number_of_axis {
            let Some(length) = self.naxis_n(axis as usize) else {
                return Err(format!(
                    "This is not a valid {}. NAXIS is {} but card NAXIS{} is missing",
                    kind,
                    number_of_axis,
                    axis + 1
                )
                .into());
            };

            if length < 0 {
                return Err(format!(
                    "This is not a valid {}. Card NAXIS{} must not be negative, but was {}",
                    kind,
                    axis + 1,
                    length
                )
                .into());
            }
        }

        Ok(())
    }

    fn read_all_cards(
        reader: &mut Box<dyn ReadSeek>,
    ) -> Result<Vec<Card>, Box<dyn Error + Send + Sync>> {
        let mut block = [0_u8; BLOCK_NUM_BYTES];
        let mut cards = vec![];

        while Self::read_block(reader, &mut block)? {
            for card in block.chunks_exact(CARD_NUM_BYTES) {
                let card = Card::try_from(
                    <&[u8; CARD_NUM_BYTES]>::try_from(card).expect("chunks_exact yields 80 bytes"),
                )?;

                let is_end = card == Card::End;
                cards.push(card);

                // Everything after END is padding.
                if is_end {
                    return Ok(cards);
                }
            }
        }

        Ok(cards)
    }

    /// Fills `block` with exactly one 2880-byte FITS block.
    ///
    /// Returns `false` at a clean end of file. `Read::read` is free to return
    /// fewer bytes than asked for even mid-file, so the read is repeated until
    /// the block is full; stopping early would misalign every following card.
    fn read_block(
        reader: &mut Box<dyn ReadSeek>,
        block: &mut [u8; BLOCK_NUM_BYTES],
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let mut filled = 0;

        while filled < BLOCK_NUM_BYTES {
            match reader.read(&mut block[filled..])? {
                0 if filled == 0 => return Ok(false),
                0 => {
                    return Err(format!(
                        "Truncated FITS header: a block is {} bytes but only {} were left",
                        BLOCK_NUM_BYTES, filled
                    )
                    .into());
                }
                bytes => filled += bytes,
            }
        }

        Ok(true)
    }
}

impl fmt::Debug for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Flexible Image Transport System (FITS) Data Unit Header:"
        )?;
        for card in &self.cards {
            if card != &Card::End {
                let value = Value::from(card);
                writeln!(
                    f,
                    "{: <8} = {: >72} / {}",
                    card.key(),
                    value.value_to_string(),
                    value.comment_to_string()
                )?;
            } else {
                write!(f, "END")?;
            }
        }

        Ok(())
    }
}
