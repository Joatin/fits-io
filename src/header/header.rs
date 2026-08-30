use crate::ascii_table::AsciiColumnFormat;
use crate::header::card::Card;
use crate::header::card_keys;
use crate::header::extension_type::ExtensionType;
use crate::header::value::Value;
use crate::header::{BayerPattern, Bitpix, ImageType, TableColumnFormat, TableNullValue};
use crate::util::ReadSeek;
use chrono::{DateTime, Utc};
use std::error::Error;
use std::fmt::Formatter;
use std::io::Read;
use std::{fmt, vec};

pub(crate) const CARD_NUM_BYTES: usize = 80;

/// FITS files are laid out in blocks of this many bytes; headers and data
/// sections are both padded up to a whole number of them.
pub(crate) const BLOCK_NUM_BYTES: usize = 2880;

/// Whether a keyword describes the table a compressed image is stored in, rather
/// than the image itself.
///
/// The `Z` keywords describe the image and are translated; the table's own
/// structural keywords, and the column definitions, have no meaning once the
/// image is unpacked.
fn describes_the_table(key: &str) -> bool {
    const STRUCTURAL: [&str; 6] = [
        card_keys::XTENSION,
        card_keys::BITPIX,
        card_keys::NAXIS,
        card_keys::PCOUNT,
        card_keys::GCOUNT,
        card_keys::TFIELDS,
    ];

    const COLUMN_PREFIXES: [&str; 9] = [
        card_keys::PREFIX_TFORM_N,
        card_keys::PREFIX_TTYPE_N,
        card_keys::PREFIX_TSCAL_N,
        card_keys::PREFIX_TZERO_N,
        card_keys::PREFIX_TNULL_N,
        card_keys::PREFIX_TDIM_N,
        card_keys::PREFIX_TUNIT_N,
        card_keys::PREFIX_TDISP_N,
        card_keys::PREFIX_TBCOL_N,
    ];

    if STRUCTURAL.contains(&key) {
        return true;
    }

    // Every `Z` keyword either describes the compression or restates a card that
    // `uncompressed` writes fresh, so none of them belong to the image.
    if key.starts_with('Z') {
        return true;
    }

    COLUMN_PREFIXES.iter().any(|prefix| {
        key.strip_prefix(prefix)
            .is_some_and(|index| !index.is_empty() && index.chars().all(|c| c.is_ascii_digit()))
    })
}

/// What a CHECKSUM card holds while the checksum that will replace it is being
/// computed.
///
/// ASCII zeros, not spaces: the encoded value carries an ASCII-zero offset in
/// every one of its sixteen characters, so a placeholder of zeros is what makes
/// swapping it for the real value change the sum by exactly that value. Spaces
/// would leave the result short by the difference.
const BLANK_CHECKSUM: &str = "0000000000000000";

/// A FITS header: the cards that describe an HDU and its data.
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

    /// The AUTHOR card: who prepared the data.
    pub fn author(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Author { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    /// The BITPIX card: the type of the values in the data section.
    ///
    /// BITPIX is mandatory, and a header lacking it is rejected when the file is
    /// opened, so this returns `Some` for any header read from a file. It is
    /// `None` only for a header built by hand and left incomplete.
    pub fn bitpix(&self) -> Option<Bitpix> {
        self.cards.iter().find_map(|card| {
            if let Card::Bitpix { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The BLANK card: the raw value that stands for an undefined pixel.
    ///
    /// The standard defines it only for the integer BITPIX types; a floating point
    /// array says the same thing with a NaN.
    pub fn blank(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::Blank { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The BLOCKED card, a deprecated hint about the file's block size.
    pub fn blocked(&self) -> Option<bool> {
        self.cards.iter().find_map(|card| {
            if let Card::Blocked { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The BSCALE card: the factor a raw array value is multiplied by.
    ///
    /// See [`Header::bscale_or_default`] for the standard's default of 1.
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

    /// The BUNIT card: the physical unit the array's values are in.
    pub fn bunit(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::BUnit { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    /// The BZERO card: the offset added to a scaled array value.
    ///
    /// See [`Header::bzero_or_default`] for the standard's default of 0.
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

    /// The DATAMAX card: the largest physical value in the array.
    pub fn data_max(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::DataMax { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The DATAMIN card: the smallest physical value in the array.
    pub fn data_min(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::DataMin { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The DATE card: when the file was written.
    pub fn date(&self) -> Option<&DateTime<Utc>> {
        self.cards.iter().find_map(|card| {
            if let Card::Date { value, .. } = card {
                Some(value)
            } else {
                None
            }
        })
    }

    /// The DATE-OBS card: when the observation was made.
    pub fn date_observed(&self) -> Option<&DateTime<Utc>> {
        self.cards.iter().find_map(|card| {
            if let Card::DateObserved { value, .. } = card {
                Some(value)
            } else {
                None
            }
        })
    }

    /// The EPOCH card, which EQUINOX supersedes.
    pub fn epoch(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::Epoch { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The EQUINOX card: the epoch of the coordinate system, in years.
    pub fn equinox(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::Equinox { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The EXTEND card: whether extensions may follow the primary HDU.
    pub fn extend(&self) -> Option<bool> {
        self.cards.iter().find_map(|card| {
            if let Card::Extend { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The EXTLEVEL card: this extension's level in a hierarchy of them.
    pub fn extension_level(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::ExtensionLevel { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The EXTNAME card: this extension's name.
    pub fn extension_name(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::ExtensionName { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    /// The EXTVER card: this extension's version.
    pub fn extension_version(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::ExtensionVersion { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The GCOUNT card: how many groups the data section holds.
    ///
    /// One for everything but a random-groups HDU.
    pub fn group_count(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::GroupCount { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The GROUPS card: whether this HDU uses the random-groups convention.
    ///
    /// See [`Header::is_random_groups`], which also checks the axis that marks it.
    pub fn groups(&self) -> Option<bool> {
        self.cards.iter().find_map(|card| {
            if let Card::Groups { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The INSTRUME card: the instrument the data came from.
    pub fn instrument(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Instrument { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    /// The NAXIS card: how many axes the data section has.
    ///
    /// NAXIS is mandatory, and a header lacking it is rejected when the file is
    /// opened, so this returns `Some` for any header read from a file. It is
    /// `None` only for a header built by hand and left incomplete.
    pub fn naxis(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::NAxis { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The OBJECT card: what was observed.
    pub fn object(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Object { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    /// The OBSERVER card: who made the observation.
    pub fn observer(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Observer { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    /// The ORIGIN card: the organisation that wrote the file.
    pub fn origin(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Origin { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    /// The PCOUNT card: how many extra values follow the array.
    ///
    /// This is a binary table's heap, or the parameters of a random-groups HDU.
    pub fn pcount(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::ParameterCount { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The REFERENC card: a publication describing the data.
    pub fn reference(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Reference { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    /// The SIMPLE card: whether the file conforms to the FITS standard.
    ///
    /// Only a primary header carries it.
    pub fn simple(&self) -> Option<bool> {
        self.cards.iter().find_map(|card| {
            if let Card::Simple { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The TELESCOP card: the telescope the data came from.
    pub fn telescope(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Telescope { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    /// The TFIELDS card: how many columns the table has.
    pub fn table_fields(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::TableFields { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The THEAP card: where a binary table's heap starts, as a byte offset
    /// into the data section.
    pub fn table_heap(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::TableHeap { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The XTENSION card: which kind of extension this is.
    ///
    /// `None` for a primary header, which is not an extension.
    pub fn extension(&self) -> Option<ExtensionType> {
        self.cards.iter().find_map(|card| {
            if let Card::Xtension { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The FOCALLEN card: the telescope's focal length.
    ///
    /// A widespread convention among astrophotography software rather than part of
    /// the standard, as are the other camera keywords near it.
    pub fn focal_length(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::FocalLength { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The EXPTIME card: how long the exposure lasted.
    pub fn exposure_time(&self) -> Option<std::time::Duration> {
        self.cards.iter().find_map(|card| {
            if let Card::ExposureTime { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The CCD-TEMP card: the sensor's temperature, in degrees Celsius.
    pub fn ccd_temperature(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::CCDTemperature { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The BAYERPAT card: the colour filter layout over the sensor.
    ///
    /// `None` for a monochrome sensor, or one that did not record the pattern.
    pub fn bayer_pattern(&self) -> Option<BayerPattern> {
        self.cards.iter().find_map(|card| {
            if let Card::BayerPattern { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The CREATOR card: the software that wrote the file.
    pub fn creator(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::Creator { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    /// The XORGSUBF card: where a subframe starts on the sensor, horizontally.
    pub fn subframe_x_position_in_binned_pixels(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::SubframeXPositionInBinnedPixels { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The YORGSUBF card: where a subframe starts on the sensor, vertically.
    pub fn subframe_y_position_in_binned_pixels(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::SubframeYPositionInBinnedPixels { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The XBINNING card: how many sensor pixels were binned into one, horizontally.
    pub fn binned_pixels_x(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::BinnedPixelsX { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The YBINNING card: how many sensor pixels were binned into one, vertically.
    pub fn binned_pixels_y(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::BinnedPixelsY { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The CCDXBIN card, another spelling of XBINNING.
    pub fn ccd_binned_pixels_x(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::CCDBinnedPixelsX { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The CCDYBIN card, another spelling of YBINNING.
    pub fn ccd_binned_pixels_y(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::CCDBinnedPixelsY { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The XPIXSZ card: the width of a pixel in microns, binning included.
    pub fn pixel_size_x_with_binning_in_microns(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::PixelSizeXWithBinningInMicrons { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The YPIXSZ card: the height of a pixel in microns, binning included.
    pub fn pixel_size_y_with_binning_in_microns(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::PixelSizeYWithBinningInMicrons { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The IMAGETYP card: whether this is a light, dark, flat or bias frame.
    pub fn image_type(&self) -> Option<&ImageType> {
        self.cards.iter().find_map(|card| {
            if let Card::ImageType { value, .. } = card {
                Some(value)
            } else {
                None
            }
        })
    }

    /// The EXPOSURE card, another spelling of EXPTIME.
    pub fn exposure(&self) -> Option<std::time::Duration> {
        self.cards.iter().find_map(|card| {
            if let Card::Exposure { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The RA card: the right ascension the telescope was pointed at.
    pub fn ra(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::Ra { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The DEC card: the declination the telescope was pointed at.
    pub fn dec(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::Dec { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The GUIDECAM card: the guide camera in use.
    pub fn guide_cam(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::GuideCam { value, .. } = card {
                Some(value.as_str())
            } else {
                None
            }
        })
    }

    /// The FOCUSPOS card: where the focuser was.
    pub fn focus_position(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::FocusPosition { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The SITELONG card: the observing site's longitude.
    pub fn site_longitude(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::SiteLongitude { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The SITELAT card: the observing site's latitude.
    pub fn site_latitude(&self) -> Option<f64> {
        self.cards.iter().find_map(|card| {
            if let Card::SiteLatitude { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The IMAGEW card: the image's width, as the writing software recorded it.
    pub fn image_width(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::ImageWidth { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The IMAGEH card: the image's height, as the writing software recorded it.
    pub fn image_height(&self) -> Option<i64> {
        self.cards.iter().find_map(|card| {
            if let Card::ImageHeight { value, .. } = card {
                Some(*value)
            } else {
                None
            }
        })
    }

    /// The CDELTn card for axis `index`: how far the world coordinate moves
    /// per pixel.
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

    /// The CROTAn card for axis `index`: the rotation between the pixel and
    /// world axes, in degrees.
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

    /// The CRPIXn card for axis `index`: the pixel that the reference value
    /// sits at, counting from 1.
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

    /// The CRVALn card for axis `index`: the world coordinate at the
    /// reference pixel.
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

    /// The CDi_j card: one element of the matrix taking pixel offsets to
    /// intermediate world coordinates.
    ///
    /// `row` and `column` count from 0, so CD1_1 is `coordinate_transform(0, 0)`.
    /// This matrix carries the scale as well as the rotation, which is why a
    /// header using it has no CDELTn cards.
    pub fn coordinate_transform(&self, row: usize, column: usize) -> Option<f64> {
        self.matrix_element("CD", row, column)
    }

    /// The PCi_j card: one element of the dimensionless matrix that rotates and
    /// skews pixel offsets, before CDELTn scales them.
    ///
    /// `row` and `column` count from 0, so PC1_1 is
    /// `coordinate_rotation_matrix(0, 0)`.
    pub fn coordinate_rotation_matrix(&self, row: usize, column: usize) -> Option<f64> {
        self.matrix_element("PC", row, column)
    }

    /// Reads one element of a two-index keyword family such as CDi_j.
    ///
    /// These are not among the keywords this crate models individually, so they
    /// arrive as plain value cards and are looked up by name.
    fn matrix_element(&self, prefix: &str, row: usize, column: usize) -> Option<f64> {
        let key = format!("{}{}_{}", prefix, row + 1, column + 1);

        self.raw_card(&key)
            .into_iter()
            .find_map(|value| match value {
                Value::Float { value, .. } => Some(value),
                // A whole number is commonly written without a decimal point.
                Value::Integer { value, .. } => Some(value as f64),
                _ => None,
            })
    }

    /// The CTYPEn card for axis `index`: what the axis measures, and the
    /// projection it uses.
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

    /// The NAXISn card for axis `index`: how long that axis is.
    ///
    /// `index` counts from 0, so NAXIS1 is `naxis_n(0)`.
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

    /// The PSCALn card for group parameter `index`.
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

    /// The PTYPEn card for group parameter `index`: what it measures.
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

    /// The PZEROn card for group parameter `index`.
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

    /// The TBCOLn card for column `index`: where the column starts within an
    /// ASCII table's row, counting from 1.
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

    /// The TDIMn card for column `index`: the shape of a multidimensional
    /// column, as written.
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

    /// The TDISPn card for column `index`: how the column is best displayed.
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

    /// The TNULLn card for column `index`: the value that marks an undefined
    /// entry in that column.
    pub fn table_null_value(&self, index: usize) -> Option<&TableNullValue> {
        self.cards.iter().find_map(|card| {
            if let Card::TableNullValueN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(value);
            };
            None
        })
    }

    /// The TSCALn card for column `index`: the factor a stored entry is
    /// multiplied by.
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

    /// The TTYPEn card for column `index`: the column's name.
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

    /// The TFORMn card for column `index`, exactly as written.
    pub fn table_format(&self, index: usize) -> Option<&str> {
        self.cards.iter().find_map(|card| {
            if let Card::TableFormatN {
                value, index: idx, ..
            } = card
                && index == *idx
            {
                return Some(value.as_str());
            };
            None
        })
    }

    /// The TFORMn card for column `index`, read as a binary table format.
    ///
    /// `None` when the card is absent or does not name a binary table format,
    /// which is the case for every ASCII table; use
    /// [`Header::ascii_column_format`] for those.
    pub fn table_column_format(&self, index: usize) -> Option<TableColumnFormat> {
        TableColumnFormat::try_from(self.table_format(index)?.to_string()).ok()
    }

    /// The TFORMn card for column `index`, read as an ASCII table format.
    ///
    /// `None` when the card is absent or does not name an ASCII table format.
    pub fn ascii_column_format(&self, index: usize) -> Option<AsciiColumnFormat> {
        AsciiColumnFormat::try_from(self.table_format(index)?.to_string()).ok()
    }

    /// The TUNITn card for column `index`: the column's physical unit.
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

    /// The TZEROn card for column `index`: the offset added after scaling.
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
    /// This is the standard's
    /// `BITPIX/8 * GCOUNT * (PCOUNT + NAXIS1 * ... * NAXISn)`. PCOUNT matters
    /// for binary tables: it is the size of the heap that follows the rows, and
    /// leaving it out puts the *next* HDU at the wrong offset in every file
    /// whose table has variable length array columns.
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

        let mut elements: usize = 1;
        for axis in 0..number_of_axis {
            // NAXISn is untrusted input: a negative or absurd length must not
            // overflow the running product.
            let Some(length) = self.naxis_n(axis as usize) else {
                return 0;
            };
            let Ok(length) = usize::try_from(length) else {
                return 0;
            };
            let Some(product) = elements.checked_mul(length) else {
                return 0;
            };
            elements = product;
        }

        // PCOUNT and GCOUNT are mandatory on extensions and absent from a
        // conforming primary header, where they are 0 and 1.
        let pcount = self.pcount().unwrap_or(0).max(0) as usize;
        let gcount = self.group_count().unwrap_or(1).max(0) as usize;

        let Some(bytes) = elements.checked_add(pcount) else {
            return 0;
        };
        let Some(bytes) = bytes.checked_mul(gcount) else {
            return 0;
        };
        let Some(bytes) = bytes.checked_mul(bitpix.byte_size()) else {
            return 0;
        };

        bytes
    }

    /// Whether this HDU is an image stored compressed inside a table.
    ///
    /// The tiled image convention keeps a compressed image in a binary table,
    /// one tile per row, and describes the image it stands for with keywords
    /// beginning `Z`. Such an HDU reads as a table unless it is decompressed;
    /// see [`BinTableHDU::read_compressed_image`](crate::hdu::BinTableHDU::read_compressed_image).
    pub fn is_compressed_image(&self) -> bool {
        matches!(
            self.raw_card(card_keys::ZIMAGE).first(),
            Some(Value::Logical { value: true, .. })
        )
    }

    /// The ZBITPIX card: the type of the values in the image once decompressed.
    pub fn compressed_bitpix(&self) -> Option<Bitpix> {
        Bitpix::try_from(self.z_integer(card_keys::ZBITPIX)?).ok()
    }

    /// The ZNAXIS card: how many axes the decompressed image has.
    pub fn compressed_naxis(&self) -> Option<i64> {
        self.z_integer(card_keys::ZNAXIS)
    }

    /// The ZNAXISn card for axis `index`: the decompressed image's length along
    /// it. `index` counts from 0.
    pub fn compressed_naxis_n(&self, index: usize) -> Option<i64> {
        self.z_integer(&format!("{}{}", card_keys::PREFIX_ZNAXIS_N, index + 1))
    }

    /// The ZTILEn card for axis `index`: how far a tile reaches along it.
    ///
    /// The convention's default is a tile one row of the image wide, which is
    /// what a header that leaves the card out means.
    pub fn compressed_tile_size(&self, index: usize) -> i64 {
        if let Some(size) = self.z_integer(&format!("{}{}", card_keys::PREFIX_ZTILE_N, index + 1)) {
            return size;
        }

        match index {
            0 => self.compressed_naxis_n(0).unwrap_or(1),
            _ => 1,
        }
    }

    /// The ZCMPTYPE card: which algorithm the tiles were compressed with.
    pub fn compression_type(&self) -> Option<&str> {
        self.cards.iter().find_map(|card| match card {
            Card::Value {
                name,
                value: Value::String { value, .. },
            } if name == card_keys::ZCMPTYPE => Some(value.as_str()),
            _ => None,
        })
    }

    /// A compression parameter, looked up by the name a ZNAMEn card gives it.
    ///
    /// The algorithms take their settings as name and value pairs rather than as
    /// keywords of their own, so Rice's block size arrives as `ZNAME1 =
    /// 'BLOCKSIZE'` with the value in `ZVAL1`.
    pub fn compression_parameter(&self, name: &str) -> Option<i64> {
        for index in 1.. {
            let key = format!("{}{}", card_keys::PREFIX_ZNAME_N, index);
            let found = self.cards.iter().find_map(|card| match card {
                Card::Value {
                    name: key_name,
                    value: Value::String { value, .. },
                } if *key_name == key => Some(value.clone()),
                _ => None,
            });

            let found = found?;

            if found.trim() == name {
                return self.z_integer(&format!("{}{}", card_keys::PREFIX_ZVAL_N, index));
            }
        }

        None
    }

    /// One of the `Z` keywords as an integer.
    ///
    /// None of them are among the keywords this crate models individually, so
    /// they arrive as plain value cards and are looked up by name.
    fn z_integer(&self, key: &str) -> Option<i64> {
        self.raw_card(key)
            .into_iter()
            .find_map(|value| match value {
                Value::Integer { value, .. } => Some(value),
                Value::Float { value, .. } => Some(value as i64),
                _ => None,
            })
    }

    /// The header the decompressed image would have.
    ///
    /// Every card that describes the table rather than the image is dropped, and
    /// BITPIX and the NAXISn cards are taken from their `Z` counterparts, so
    /// that the result describes the image the HDU stands for. Anything else the
    /// header carried — WCS keywords especially — comes across untouched.
    pub fn uncompressed(&self) -> Self {
        let mut header = Self {
            cards: self
                .cards
                .iter()
                .filter(|card| !describes_the_table(&card.key()))
                .cloned()
                .collect(),
        };

        header.remove_prefixed(card_keys::PREFIX_NAXIS_N);

        if let Some(bitpix) = self.compressed_bitpix() {
            header.set(Card::Bitpix {
                value: bitpix,
                comment: None,
            });
        }

        let axes = self.compressed_naxis().unwrap_or(0).max(0);
        header.set(Card::NAxis {
            value: axes,
            comment: None,
        });
        for axis in 0..axes as usize {
            header.set(Card::NAxisN {
                index: axis,
                value: self.compressed_naxis_n(axis).unwrap_or(0),
                comment: None,
            });
        }

        header
    }

    /// Whether this HDU uses the random-groups convention.
    ///
    /// Such an HDU's data section is not an image but GCOUNT groups, each one a
    /// run of PCOUNT parameters followed by an array. The convention is marked
    /// by `GROUPS = T`, and by a first axis of length zero standing in for the
    /// axis the groups occupy.
    pub fn is_random_groups(&self) -> bool {
        self.groups() == Some(true) && self.naxis_n(0) == Some(0)
    }

    /// How many values each group's array holds, for a random-groups HDU.
    ///
    /// The first axis is the placeholder that marks the convention, so the array
    /// is the axes after it.
    pub(crate) fn group_array_len(&self) -> usize {
        let Some(axes) = self.naxis() else {
            return 0;
        };

        let mut elements = 1_usize;
        for axis in 1..axes.max(0) as usize {
            let length = self.naxis_n(axis).unwrap_or(0).max(0) as usize;
            let Some(product) = elements.checked_mul(length) else {
                return 0;
            };
            elements = product;
        }

        elements
    }

    /// How many two-dimensional planes an image HDU's data section holds.
    ///
    /// The first two axes are the image; every axis beyond them multiplies the
    /// number of images, so a NAXIS = 4 array with NAXIS3 = 2 and NAXIS4 = 3
    /// holds six planes, not two. An HDU with fewer than two axes holds no
    /// image at all.
    pub(crate) fn image_plane_count(&self) -> usize {
        let Some(axes) = self.naxis() else {
            return 0;
        };

        if axes < 2 {
            return 0;
        }

        let mut planes = 1_usize;
        for axis in 2..axes as usize {
            let length = self.naxis_n(axis).unwrap_or(0).max(0) as usize;

            // A zero-length axis means no data at all, not "ignore this axis".
            let Some(product) = planes.checked_mul(length) else {
                return 0;
            };
            planes = product;
        }

        planes
    }

    /// Byte offset of a binary table's heap from the start of its data section.
    ///
    /// THEAP names it explicitly; a table without that card puts the heap
    /// directly after the last row.
    pub(crate) fn table_heap_offset(&self) -> usize {
        if let Some(offset) = self.table_heap()
            && let Ok(offset) = usize::try_from(offset)
        {
            return offset;
        }

        let rows = |axis| self.naxis_n(axis).unwrap_or(0).max(0) as usize;
        rows(0).saturating_mul(rows(1))
    }

    /// Renders this header as the bytes it occupies in a file.
    ///
    /// The result is always a whole number of 2880-byte blocks, padded with
    /// spaces, and always ends with an END card — a header without one is not a
    /// header a reader can find the end of.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.bytes_len());

        for card in &self.cards {
            if card == &Card::End {
                break;
            }
            bytes.extend_from_slice(&card.to_bytes());
        }

        bytes.extend_from_slice(&Card::End.to_bytes());

        let padding = (BLOCK_NUM_BYTES - bytes.len() % BLOCK_NUM_BYTES) % BLOCK_NUM_BYTES;
        bytes.resize(bytes.len() + padding, b' ');

        bytes
    }

    /// Writes the DATASUM and CHECKSUM cards for an HDU whose data section is
    /// `data`.
    ///
    /// CHECKSUM covers the whole HDU including its own card, so it cannot be
    /// known until the header has been rendered. It is set to blanks here and
    /// filled in by [`Header::checksummed_bytes`] once there is a header to sum.
    pub(crate) fn set_checksum_placeholders(&mut self, data: &[u8]) {
        self.set(Card::Value {
            name: card_keys::DATASUM.to_string(),
            value: Value::String {
                value: crate::checksum::sum32(data, 0).to_string(),
                comment: Some("checksum of the data section".into()),
            },
        });
        self.set(Card::Value {
            name: card_keys::CHECKSUM.to_string(),
            value: Value::String {
                value: BLANK_CHECKSUM.to_string(),
                comment: Some("checksum of the whole HDU".into()),
            },
        });
    }

    /// This header rendered with a CHECKSUM that is correct for it and `data`.
    ///
    /// The card is written blank, the whole HDU is summed, and the card is then
    /// filled in with the complement of that sum — so that summing the finished
    /// HDU gives all ones. The blank value and the final one are the same width,
    /// so filling it in does not move anything.
    pub(crate) fn checksummed_bytes(&self, data: &[u8]) -> Vec<u8> {
        let mut header = self.clone();
        header.set_checksum_placeholders(data);

        let blank = header.to_bytes();

        let sum = crate::checksum::sum32(data, crate::checksum::sum32(&blank, 0));
        let checksum = crate::checksum::encode(crate::checksum::complement(sum));

        header.set(Card::Value {
            name: card_keys::CHECKSUM.to_string(),
            value: Value::String {
                value: checksum,
                comment: Some("checksum of the whole HDU".into()),
            },
        });

        let bytes = header.to_bytes();
        debug_assert_eq!(
            bytes.len(),
            blank.len(),
            "filling in the checksum must not change the header's length"
        );

        bytes
    }

    /// Sets the NAXISn card for axis `index`, which counts from 0.
    ///
    /// # Errors
    ///
    /// Returns an error for a negative length, which no axis can have.
    pub fn set_naxis_n(
        &mut self,
        index: usize,
        length: i64,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if length < 0 {
            return Err(format!("An axis cannot be {} long", length).into());
        }

        self.set(Card::NAxisN {
            index,
            value: length,
            comment: None,
        });

        Ok(())
    }

    /// Checks that this header describes a data section of `actual` bytes.
    ///
    /// The header is the only thing that says how to read the data after it, so
    /// one that disagrees with what follows produces a file nothing can read:
    /// the next HDU is looked for at the wrong offset, and the array comes back
    /// the wrong shape. Setting an image or a table keeps the two in step, but a
    /// caller who edits NAXISn through [`Header::header_mut`] can put them out
    /// of step again, and this is where that is caught.
    ///
    /// This can only catch an HDU that carries its own data. Where the data is
    /// still in a file, the header is what says how much of it to read, so the
    /// two cannot disagree — a header edited to describe more than the file
    /// holds fails when the read runs off the end instead.
    ///
    /// [`Header::header_mut`]: crate::hdu::HDU::header_mut
    pub(crate) fn validate_against_data(
        &self,
        actual: usize,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let declared = self.data_bytes_len();

        // The data section is padded out to whole blocks, so anything from the
        // declared length up to the end of its last block is consistent.
        let padded = declared.div_ceil(BLOCK_NUM_BYTES) * BLOCK_NUM_BYTES;

        if actual < declared || actual > padded {
            return Err(format!(
                "This header describes {} bytes of data, but the HDU holds {}. A header that \
                 disagrees with its data produces a file that cannot be read back.",
                declared, actual
            )
            .into());
        }

        Ok(())
    }

    /// This header with its mandatory cards present and in the order the FITS
    /// standard requires.
    ///
    /// The standard is strict about the front of a header: a primary header
    /// opens with SIMPLE, BITPIX, NAXIS and then one NAXISn per axis, and an
    /// extension header opens with XTENSION and continues through PCOUNT and
    /// GCOUNT. A reader is entitled to reject anything else, so a header that is
    /// being written out is put in that order here rather than left however it
    /// was assembled.
    ///
    /// Missing mandatory cards are filled in with the values the standard
    /// defines: a header built from nothing has no SIMPLE card at all, and a
    /// file written from one would not be readable.
    ///
    /// `extension` names the kind of extension this header belongs to, or
    /// `None` for the primary header.
    pub(crate) fn conformed(&self, extension: Option<ExtensionType>) -> Self {
        let mut mandatory = Vec::new();

        match extension {
            None => mandatory.push(Card::Simple {
                // A file this crate wrote conforms to the standard, so SIMPLE is
                // true even if the header it came from said otherwise.
                value: true,
                comment: self.comment_for(card_keys::SIMPLE),
            }),
            Some(extension) => mandatory.push(Card::Xtension {
                value: extension,
                comment: self.comment_for(card_keys::XTENSION),
            }),
        }

        mandatory.push(Card::Bitpix {
            value: self.bitpix().unwrap_or(Bitpix::U8),
            comment: self.comment_for(card_keys::BITPIX),
        });

        let axes = self.naxis().unwrap_or(0).max(0);
        mandatory.push(Card::NAxis {
            value: axes,
            comment: self.comment_for(card_keys::NAXIS),
        });

        for axis in 0..axes as usize {
            mandatory.push(Card::NAxisN {
                index: axis,
                value: self.naxis_n(axis).unwrap_or(0),
                comment: self.comment_for(&format!("{}{}", card_keys::PREFIX_NAXIS_N, axis + 1)),
            });
        }

        // PCOUNT and GCOUNT are mandatory on every extension and are not written
        // in a conforming primary header.
        if extension.is_some() {
            mandatory.push(Card::ParameterCount {
                value: self.pcount().unwrap_or(0),
                comment: self.comment_for(card_keys::PCOUNT),
            });
            mandatory.push(Card::GroupCount {
                value: self.group_count().unwrap_or(1),
                comment: self.comment_for(card_keys::GCOUNT),
            });
        }

        // A table's TFIELDS belongs immediately after GCOUNT.
        if matches!(
            extension,
            Some(ExtensionType::BinTable | ExtensionType::AsciiTable)
        ) {
            mandatory.push(Card::TableFields {
                value: self.table_fields().unwrap_or(0),
                comment: self.comment_for(card_keys::TFIELDS),
            });
        }

        let placed: Vec<String> = mandatory.iter().map(Card::key).collect();

        // Everything else keeps the order it already had, minus the cards that
        // have just been placed at the front and any END, which `to_bytes` adds.
        let rest = self
            .cards
            .iter()
            .filter(|card| **card != Card::End && !placed.contains(&card.key()));

        Self {
            cards: mandatory.iter().cloned().chain(rest.cloned()).collect(),
        }
    }

    /// The comment on the existing card for `key`, so that rewriting a header
    /// does not throw away what its cards said about themselves.
    fn comment_for(&self, key: &str) -> Option<String> {
        self.cards
            .iter()
            .find(|card| card.key() == key)
            .and_then(|card| match Value::from(card) {
                Value::Integer { comment, .. }
                | Value::Float { comment, .. }
                | Value::Logical { comment, .. }
                | Value::String { comment, .. } => comment,
                _ => None,
            })
    }

    /// Replaces the card for `key`, or adds it before the END card.
    ///
    /// Writing an image means bringing BITPIX and the NAXISn cards into line
    /// with the data, and those cards are already there in a header that was
    /// read from a file.
    pub(crate) fn set(&mut self, card: Card) {
        let key = card.key();

        if let Some(existing) = self.cards.iter_mut().find(|existing| existing.key() == key) {
            *existing = card;
            return;
        }

        match self.cards.iter().position(|card| card == &Card::End) {
            Some(end) => self.cards.insert(end, card),
            None => self.cards.push(card),
        }
    }

    /// Removes every indexed card whose keyword starts with `prefix`, such as
    /// every TFORMn.
    ///
    /// The index has to be there: `NAXIS` is not one of the `NAXISn` cards, and
    /// removing it along with them would leave a header that no longer says how
    /// many axes it has.
    pub(crate) fn remove_prefixed(&mut self, prefix: &str) {
        self.cards.retain(|card| {
            let key = card.key();
            let Some(index) = key.strip_prefix(prefix) else {
                return true;
            };

            !(!index.is_empty() && index.chars().all(|c| c.is_ascii_digit()))
        });
    }

    /// Every card with the keyword `key`, as raw values.
    ///
    /// Most keywords appear once, but COMMENT and HISTORY may repeat, and an
    /// unrecognised keyword can appear as often as the writer liked.
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
            // `as_chunks` hands back fixed-size arrays, so there is nothing to
            // convert and no length to assert.
            for card in block.as_chunks::<CARD_NUM_BYTES>().0 {
                let card = Card::try_from(card)?;

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
