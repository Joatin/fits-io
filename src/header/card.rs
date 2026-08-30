use crate::header::extension_type::ExtensionType;
use crate::header::header::CARD_NUM_BYTES;
use crate::header::value::Value;
use crate::header::{BayerPattern, Bitpix, ImageType, TableNullValue, card_keys};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::error::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum Card {
    Author {
        value: String,
        comment: Option<String>,
    },
    Bitpix {
        value: Bitpix,
        comment: Option<String>,
    },
    Blank {
        value: i64,
        comment: Option<String>,
    },
    Blocked {
        value: bool,
        comment: Option<String>,
    },
    BScale {
        value: f64,
        comment: Option<String>,
    },
    BUnit {
        value: String,
        comment: Option<String>,
    },
    BZero {
        value: f64,
        comment: Option<String>,
    },
    CoordinateDeltaN {
        index: usize,
        value: f64,
        comment: Option<String>,
    },
    CoordinateRotationN {
        index: usize,
        value: f64,
        comment: Option<String>,
    },
    CoordinateReferencePixelN {
        index: usize,
        value: f64,
        comment: Option<String>,
    },
    CoordinateValueAtPixelN {
        index: usize,
        value: f64,
        comment: Option<String>,
    },
    CoordinateAxisNameN {
        index: usize,
        value: String,
        comment: Option<String>,
    },
    Comment(String),
    DataMax {
        value: f64,
        comment: Option<String>,
    },
    DataMin {
        value: f64,
        comment: Option<String>,
    },
    Date {
        value: DateTime<Utc>,
        comment: Option<String>,
    },
    DateObserved {
        value: DateTime<Utc>,
        comment: Option<String>,
    },
    End,
    Epoch {
        value: f64,
        comment: Option<String>,
    },
    Equinox {
        value: f64,
        comment: Option<String>,
    },
    Extend {
        value: bool,
        comment: Option<String>,
    },
    ExtensionLevel {
        value: i64,
        comment: Option<String>,
    },
    ExtensionName {
        value: String,
        comment: Option<String>,
    },
    ExtensionVersion {
        value: i64,
        comment: Option<String>,
    },
    GroupCount {
        value: i64,
        comment: Option<String>,
    },
    Groups {
        value: bool,
        comment: Option<String>,
    },
    History(String),
    Instrument {
        value: String,
        comment: Option<String>,
    },
    NAxis {
        value: i64,
        comment: Option<String>,
    },
    NAxisN {
        index: usize,
        value: i64,
        comment: Option<String>,
    },
    Object {
        value: String,
        comment: Option<String>,
    },
    Observer {
        value: String,
        comment: Option<String>,
    },
    Origin {
        value: String,
        comment: Option<String>,
    },
    ParameterCount {
        value: i64,
        comment: Option<String>,
    },
    ParameterScalingFactorN {
        index: usize,
        value: f64,
        comment: Option<String>,
    },
    ParameterTypeN {
        index: usize,
        value: String,
        comment: Option<String>,
    },
    ParameterScalingZeroPointN {
        index: usize,
        value: f64,
        comment: Option<String>,
    },
    Reference {
        value: String,
        comment: Option<String>,
    },
    Simple {
        value: bool,
        comment: Option<String>,
    },
    TableColumnN {
        index: usize,
        value: i64,
        comment: Option<String>,
    },
    TableDimensionsN {
        index: usize,
        value: String,
        comment: Option<String>,
    },
    TableDisplayFormatN {
        index: usize,
        value: String,
        comment: Option<String>,
    },
    Telescope {
        value: String,
        comment: Option<String>,
    },
    TableFields {
        value: i64,
        comment: Option<String>,
    },
    TableHeap {
        value: i64,
        comment: Option<String>,
    },
    TableNullValueN {
        index: usize,
        value: TableNullValue,
        comment: Option<String>,
    },
    TableScalingFactorN {
        index: usize,
        value: f64,
        comment: Option<String>,
    },
    TableTypeN {
        index: usize,
        value: String,
        comment: Option<String>,
    },
    TableUnitN {
        index: usize,
        value: String,
        comment: Option<String>,
    },
    /// TFORMn, kept as written.
    ///
    /// The same code means different things in the two kinds of table — `E` is
    /// a 32-bit float in a binary table and a fixed-width decimal field in an
    /// ASCII one — and a card does not know which kind of table it belongs to.
    /// Interpreting it is left to the reader, which does; see
    /// [`Header::table_column_format`](crate::header::Header::table_column_format)
    /// and
    /// [`Header::ascii_column_format`](crate::header::Header::ascii_column_format).
    TableFormatN {
        index: usize,
        value: String,
        comment: Option<String>,
    },
    TableScalingZeroPointN {
        index: usize,
        value: f64,
        comment: Option<String>,
    },
    Xtension {
        value: ExtensionType,
        comment: Option<String>,
    },
    FocalLength {
        value: f64,
        comment: Option<String>,
    },
    ExposureTime {
        value: std::time::Duration,
        comment: Option<String>,
    },
    CCDTemperature {
        value: f64,
        comment: Option<String>,
    },
    BayerPattern {
        value: BayerPattern,
        comment: Option<String>,
    },
    Creator {
        value: String,
        comment: Option<String>,
    },
    SubframeXPositionInBinnedPixels {
        value: i64,
        comment: Option<String>,
    },
    SubframeYPositionInBinnedPixels {
        value: i64,
        comment: Option<String>,
    },
    BinnedPixelsX {
        value: i64,
        comment: Option<String>,
    },
    BinnedPixelsY {
        value: i64,
        comment: Option<String>,
    },
    CCDBinnedPixelsX {
        value: i64,
        comment: Option<String>,
    },
    CCDBinnedPixelsY {
        value: i64,
        comment: Option<String>,
    },
    PixelSizeXWithBinningInMicrons {
        value: f64,
        comment: Option<String>,
    },
    PixelSizeYWithBinningInMicrons {
        value: f64,
        comment: Option<String>,
    },
    ImageType {
        value: ImageType,
        comment: Option<String>,
    },
    Exposure {
        value: std::time::Duration,
        comment: Option<String>,
    },
    Ra {
        value: f64,
        comment: Option<String>,
    },
    Dec {
        value: f64,
        comment: Option<String>,
    },
    GuideCam {
        value: String,
        comment: Option<String>,
    },
    FocusPosition {
        value: i64,
        comment: Option<String>,
    },
    SiteLongitude {
        value: f64,
        comment: Option<String>,
    },
    SiteLatitude {
        value: f64,
        comment: Option<String>,
    },
    ImageWidth {
        value: i64,
        comment: Option<String>,
    },
    ImageHeight {
        value: i64,
        comment: Option<String>,
    },
    Value {
        name: String,
        value: Value,
    },
    Continuation {
        string: Option<String>,
        comment: Option<String>,
    },
    Hierarch {
        name: String,
        value: Value,
    },
    Space,
    Undefined(String),
}

impl Card {
    pub fn try_from(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let key_word = std::str::from_utf8(buf[..8].trim_ascii())?;
        match key_word {
            "" => parse_empty_keyword_card(buf),
            card_keys::AUTHOR => Ok(Self::parse_author(buf)?),
            card_keys::BITPIX => Ok(Self::parse_bitpix(buf)?),
            card_keys::BLANK => Ok(Self::parse_blank(buf)?),
            card_keys::BLOCKED => Ok(Self::parse_blocked(buf)?),
            card_keys::BSCALE => Ok(Self::parse_bscale(buf)?),
            card_keys::BUNIT => Ok(Self::parse_bunit(buf)?),
            card_keys::BZERO => Ok(Self::parse_bzero(buf)?),
            card_keys::DATAMAX => Ok(Self::parse_data_max(buf)?),
            card_keys::DATAMIN => Ok(Self::parse_data_min(buf)?),
            card_keys::DATE => Ok(Self::parse_date(buf)?),
            card_keys::DATE_OBS => Ok(Self::parse_date_observed(buf)?),
            card_keys::EPOCH => Ok(Self::parse_epoch(buf)?),
            card_keys::EQUINOX => Ok(Self::parse_equinox(buf)?),
            card_keys::EXTEND => Ok(Self::parse_extend(buf)?),
            card_keys::EXTLEVEL => Ok(Self::parse_extension_level(buf)?),
            card_keys::EXTNAME => Ok(Self::parse_extension_name(buf)?),
            card_keys::EXTVER => Ok(Self::parse_extension_version(buf)?),
            card_keys::GCOUNT => Ok(Self::parse_group_count(buf)?),
            card_keys::GROUPS => Ok(Self::parse_groups(buf)?),
            card_keys::INSTRUME => Ok(Self::parse_instrument(buf)?),
            card_keys::NAXIS => Ok(Self::parse_naxis(buf)?),
            card_keys::OBJECT => Ok(Self::parse_object(buf)?),
            card_keys::OBSERVER => Ok(Self::parse_observer(buf)?),
            card_keys::ORIGIN => Ok(Self::parse_origin(buf)?),
            card_keys::PCOUNT => Ok(Self::parse_pcount(buf)?),
            card_keys::REFERENC => Ok(Self::parse_reference(buf)?),
            card_keys::SIMPLE => Ok(Self::parse_simple(buf)?),
            card_keys::TELESCOP => Ok(Self::parse_telescope(buf)?),
            card_keys::TFIELDS => Ok(Self::parse_table_fields(buf)?),
            card_keys::THEAP => Ok(Self::parse_table_heap(buf)?),
            card_keys::XTENSION => Ok(Self::parse_xtension(buf)?),
            card_keys::FOCALLEN => Ok(Self::parse_focal_length(buf)?),
            card_keys::EXPTIME => Ok(Self::parse_exposure_time(buf)?),
            card_keys::CCD_TEMP => Ok(Self::parse_ccd_temperature(buf)?),
            card_keys::BAYERPAT => Ok(Self::parse_bayer_pattern(buf)?),
            card_keys::CREATOR => Ok(Self::parse_creator(buf)?),
            card_keys::XORGSUBF => Ok(Self::parse_subframe_x_position_in_binned_pixels(buf)?),
            card_keys::YORGSUBF => Ok(Self::parse_subframe_y_position_in_binned_pixels(buf)?),
            card_keys::XBINNING => Ok(Self::parse_binned_pixels_x(buf)?),
            card_keys::YBINNING => Ok(Self::parse_binned_pixels_y(buf)?),
            card_keys::CCDXBIN => Ok(Self::parse_ccd_binned_pixels_x(buf)?),
            card_keys::CCDYBIN => Ok(Self::parse_ccd_binned_pixels_y(buf)?),
            card_keys::XPIXSZ => Ok(Self::parse_pixel_size_x_with_binning_in_microns(buf)?),
            card_keys::YPIXSZ => Ok(Self::parse_pixel_size_y_with_binning_in_microns(buf)?),
            card_keys::IMAGETYP => Ok(Self::parse_image_type(buf)?),
            card_keys::EXPOSURE => Ok(Self::parse_exposure(buf)?),
            card_keys::RA => Ok(Self::parse_ra(buf)?),
            card_keys::DEC => Ok(Self::parse_dec(buf)?),
            card_keys::GUIDECAM => Ok(Self::parse_guide_cam(buf)?),
            card_keys::FOCUSPOS => Ok(Self::parse_focus_position(buf)?),
            card_keys::SITELONG => Ok(Self::parse_site_longitude(buf)?),
            card_keys::SITELAT => Ok(Self::parse_site_latitude(buf)?),
            card_keys::IMAGEW => Ok(Self::parse_image_width(buf)?),
            card_keys::IMAGEH => Ok(Self::parse_image_height(buf)?),
            card_keys::COMMENT => Ok(Card::Comment(parse_comment_text(&buf[8..])?)),
            card_keys::HISTORY => Ok(Card::History(parse_comment_text(&buf[8..])?)),
            "CONTINUE" => parse_continuation(buf),
            "HIERARCH" => parse_hierarch(buf),
            card_keys::END => Ok(Card::End),
            key if key.starts_with(card_keys::PREFIX_CDELT_N) => {
                Ok(Self::parse_coordinate_delta(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_CROTA_N) => {
                Ok(Self::parse_coordinate_rotation(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_CRPIX_N) => {
                Ok(Self::parse_coordinate_reference_pixel(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_CRVAL_N) => {
                Ok(Self::parse_coordinate_value_at_pixel(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_CTYPE_N) => {
                Ok(Self::parse_coordinate_axis_name(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_NAXIS_N) => Ok(Self::parse_naxis_n(key, buf)?),
            key if key.starts_with(card_keys::PREFIX_PSCAL_N) => {
                Ok(Self::parse_parameter_scaling_factor(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_PTYPE_N) => {
                Ok(Self::parse_parameter_type(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_PZERO_N) => {
                Ok(Self::parse_parameter_scaling_zero_point(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_TBCOL_N) => {
                Ok(Self::parse_table_column(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_TDIM_N) => {
                Ok(Self::parse_table_dimensions(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_TFORM_N) => {
                Ok(Self::parse_table_column_format(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_TDISP_N) => {
                Ok(Self::parse_table_display_format(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_TNULL_N) => {
                Ok(Self::parse_table_null_value(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_TSCAL_N) => {
                Ok(Self::parse_table_scaling_factor(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_TTYPE_N) => {
                Ok(Self::parse_table_type(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_TUNIT_N) => {
                Ok(Self::parse_table_unit(key, buf)?)
            }
            key if key.starts_with(card_keys::PREFIX_TZERO_N) => {
                Ok(Self::parse_table_scaling_zero_point(key, buf)?)
            }
            _ => {
                if b"= " == &buf[8..10] {
                    Ok(Card::Value {
                        name: key_word.to_string(),
                        value: parse_value(&buf[10..])?,
                    })
                } else {
                    Ok(Card::Undefined(String::from_utf8_lossy(buf).into_owned()))
                }
            }
        }
    }

    fn parse_author(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            Ok(Card::Author { value, comment })
        } else {
            Err("Invalid author data format".into())
        }
    }

    fn parse_bitpix(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::Bitpix {
                value: value.try_into()?,
                comment,
            })
        } else {
            Err("Invalid bitpix data format".into())
        }
    }

    fn parse_blank(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::Blank { value, comment })
        } else {
            Err("Invalid blank data format".into())
        }
    }

    fn parse_blocked(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Logical { value, comment } = value {
            Ok(Card::Blocked { value, comment })
        } else {
            Err("Invalid blocked data format".into())
        }
    }

    fn parse_bscale(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            Ok(Card::BScale { value, comment })
        } else {
            Err("Invalid bscale data format".into())
        }
    }

    fn parse_bunit(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            Ok(Card::BUnit { value, comment })
        } else {
            Err("Invalid bunit data format".into())
        }
    }

    fn parse_bzero(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            Ok(Card::BZero { value, comment })
        } else {
            Err("Invalid bzero data format".into())
        }
    }

    fn parse_data_max(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            Ok(Card::DataMax { value, comment })
        } else {
            Err("Invalid data max data format".into())
        }
    }

    fn parse_data_min(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            Ok(Card::DataMin { value, comment })
        } else {
            Err("Invalid data min data format".into())
        }
    }

    fn parse_date(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            let value: NaiveDateTime = value.parse()?;
            let value = value.and_utc();
            Ok(Card::Date { value, comment })
        } else {
            Err("Invalid date data format".into())
        }
    }

    fn parse_date_observed(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            let value: NaiveDateTime = value.parse()?;
            let value = value.and_utc();
            Ok(Card::DateObserved { value, comment })
        } else {
            Err("Invalid date observed data format".into())
        }
    }

    fn parse_epoch(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            Ok(Card::Epoch { value, comment })
        } else {
            Err("Invalid epoch data format".into())
        }
    }

    fn parse_equinox(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            Ok(Card::Equinox { value, comment })
        } else {
            Err("Invalid equinox data format".into())
        }
    }

    fn parse_extend(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Logical { value, comment } = value {
            Ok(Card::Extend { value, comment })
        } else {
            Err("Invalid extend data format".into())
        }
    }

    fn parse_extension_level(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::ExtensionLevel { value, comment })
        } else {
            Err("Invalid extension level data format".into())
        }
    }

    fn parse_extension_name(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            Ok(Card::ExtensionName { value, comment })
        } else {
            Err("Invalid extension name data format".into())
        }
    }

    fn parse_extension_version(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::ExtensionVersion { value, comment })
        } else {
            Err("Invalid extension version data format".into())
        }
    }

    fn parse_group_count(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::GroupCount { value, comment })
        } else {
            Err("Invalid group count data format".into())
        }
    }

    fn parse_groups(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Logical { value, comment } = value {
            Ok(Card::Groups { value, comment })
        } else {
            Err("Invalid groups data format".into())
        }
    }

    fn parse_instrument(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            Ok(Card::Instrument { value, comment })
        } else {
            Err("Invalid instrument data format".into())
        }
    }

    fn parse_naxis(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::NAxis { value, comment })
        } else {
            Err("Invalid naxis data format".into())
        }
    }

    fn parse_object(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            Ok(Card::Object { value, comment })
        } else {
            Err("Invalid object data format".into())
        }
    }

    fn parse_observer(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            Ok(Card::Observer { value, comment })
        } else {
            Err("Invalid observer data format".into())
        }
    }

    fn parse_origin(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            Ok(Card::Origin { value, comment })
        } else {
            Err("Invalid origin data format".into())
        }
    }

    fn parse_pcount(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::ParameterCount { value, comment })
        } else {
            Err("Invalid pcount data format".into())
        }
    }

    fn parse_reference(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            Ok(Card::Reference { value, comment })
        } else {
            Err("Invalid referenc data format".into())
        }
    }

    fn parse_simple(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Logical { value, comment } = value {
            Ok(Card::Simple { value, comment })
        } else {
            Err("Invalid simple data format".into())
        }
    }

    fn parse_telescope(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            Ok(Card::Telescope { value, comment })
        } else {
            Err("Invalid telescope data format".into())
        }
    }

    fn parse_table_fields(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::TableFields { value, comment })
        } else {
            Err("Invalid tfields data format".into())
        }
    }

    fn parse_table_heap(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::TableHeap { value, comment })
        } else {
            Err("Invalid theap data format".into())
        }
    }

    fn parse_xtension(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            Ok(Card::Xtension {
                value: value.try_into()?,
                comment,
            })
        } else {
            Err("Invalid xtension data format".into())
        }
    }

    fn parse_focal_length(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            Ok(Card::FocalLength { value, comment })
        } else {
            Err("Invalid focallen data format".into())
        }
    }

    fn parse_exposure_time(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            let value = std::time::Duration::from_secs_f64(value);
            Ok(Card::ExposureTime { value, comment })
        } else {
            Err("Invalid exptime data format".into())
        }
    }

    fn parse_ccd_temperature(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            Ok(Card::CCDTemperature { value, comment })
        } else {
            Err("Invalid ccd-temp data format".into())
        }
    }

    fn parse_bayer_pattern(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            Ok(Card::BayerPattern {
                value: value.try_into()?,
                comment,
            })
        } else {
            Err("Invalid bayer pattern data format".into())
        }
    }

    fn parse_creator(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            Ok(Card::Creator { value, comment })
        } else {
            Err("Invalid creator data format".into())
        }
    }

    fn parse_subframe_x_position_in_binned_pixels(
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::SubframeXPositionInBinnedPixels { value, comment })
        } else {
            Err("Invalid XORGSUBF data format".into())
        }
    }

    fn parse_subframe_y_position_in_binned_pixels(
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::SubframeXPositionInBinnedPixels { value, comment })
        } else {
            Err("Invalid YORGSUBF data format".into())
        }
    }

    fn parse_binned_pixels_x(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::BinnedPixelsX { value, comment })
        } else {
            Err("Invalid XBINNING data format".into())
        }
    }

    fn parse_binned_pixels_y(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::BinnedPixelsY { value, comment })
        } else {
            Err("Invalid YBINNING data format".into())
        }
    }

    fn parse_ccd_binned_pixels_x(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::CCDBinnedPixelsX { value, comment })
        } else {
            Err("Invalid CCDXBIN data format".into())
        }
    }

    fn parse_ccd_binned_pixels_y(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::CCDBinnedPixelsY { value, comment })
        } else {
            Err("Invalid CCDYBIN data format".into())
        }
    }

    fn parse_pixel_size_x_with_binning_in_microns(
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            Ok(Card::PixelSizeXWithBinningInMicrons { value, comment })
        } else {
            Err("Invalid XPIXSZ data format".into())
        }
    }

    fn parse_pixel_size_y_with_binning_in_microns(
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            Ok(Card::PixelSizeYWithBinningInMicrons { value, comment })
        } else {
            Err("Invalid YPIXSZ data format".into())
        }
    }

    fn parse_image_type(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            Ok(Card::ImageType {
                value: value.into(),
                comment,
            })
        } else {
            Err("Invalid IMAGETYP data format".into())
        }
    }

    fn parse_exposure(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            let value = std::time::Duration::from_secs_f64(value);
            Ok(Card::Exposure { value, comment })
        } else {
            Err("Invalid EXPOSURE data format".into())
        }
    }

    fn parse_ra(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            Ok(Card::Ra { value, comment })
        } else {
            Err("Invalid RA data format".into())
        }
    }

    fn parse_dec(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            Ok(Card::Dec { value, comment })
        } else {
            Err("Invalid DEC data format".into())
        }
    }

    fn parse_guide_cam(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::String { value, comment } = value {
            Ok(Card::GuideCam { value, comment })
        } else {
            Err("Invalid GUIDECAM data format".into())
        }
    }

    fn parse_focus_position(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::FocusPosition { value, comment })
        } else {
            Err("Invalid FOCUSPOS data format".into())
        }
    }

    fn parse_site_longitude(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            Ok(Card::SiteLongitude { value, comment })
        } else {
            Err("Invalid SITELONG data format".into())
        }
    }

    fn parse_site_latitude(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Some((value, comment)) = as_float(value) {
            Ok(Card::SiteLatitude { value, comment })
        } else {
            Err("Invalid SITELAT data format".into())
        }
    }

    fn parse_image_width(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::ImageWidth { value, comment })
        } else {
            Err("Invalid IMAGEW data format".into())
        }
    }

    fn parse_image_height(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Integer { value, comment } = value {
            Ok(Card::ImageHeight { value, comment })
        } else {
            Err("Invalid IMAGEH data format".into())
        }
    }

    fn parse_coordinate_delta(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_CDELT_N)?;

        if let Some((value, comment)) = as_float(value) {
            Ok(Card::CoordinateDeltaN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid cdeltn data format".into())
        }
    }

    fn parse_coordinate_rotation(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_CROTA_N)?;

        if let Some((value, comment)) = as_float(value) {
            Ok(Card::CoordinateRotationN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid crotan data format".into())
        }
    }

    fn parse_coordinate_reference_pixel(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_CRPIX_N)?;

        if let Some((value, comment)) = as_float(value) {
            Ok(Card::CoordinateReferencePixelN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid crpixn data format".into())
        }
    }

    fn parse_coordinate_value_at_pixel(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_CRVAL_N)?;

        if let Some((value, comment)) = as_float(value) {
            Ok(Card::CoordinateValueAtPixelN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid crvaln data format".into())
        }
    }

    fn parse_coordinate_axis_name(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_CTYPE_N)?;

        if let Value::String { value, comment } = value {
            Ok(Card::CoordinateAxisNameN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid crvaln data format".into())
        }
    }

    fn parse_naxis_n(key: &str, buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_NAXIS_N)?;

        if let Value::Integer { value, comment } = value {
            Ok(Card::NAxisN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid naxisn data format".into())
        }
    }

    fn parse_parameter_scaling_factor(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_PSCAL_N)?;

        if let Some((value, comment)) = as_float(value) {
            Ok(Card::ParameterScalingFactorN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid PSCALN data format".into())
        }
    }

    fn parse_parameter_type(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_PTYPE_N)?;

        if let Value::String { value, comment } = value {
            Ok(Card::ParameterTypeN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid PTYPEN data format".into())
        }
    }

    fn parse_parameter_scaling_zero_point(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_PZERO_N)?;

        if let Some((value, comment)) = as_float(value) {
            Ok(Card::ParameterScalingZeroPointN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid PZERON data format".into())
        }
    }

    fn parse_table_column(key: &str, buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_TBCOL_N)?;

        if let Value::Integer { value, comment } = value {
            Ok(Card::TableColumnN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid TBCOLN data format".into())
        }
    }

    fn parse_table_dimensions(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = key.replace(card_keys::PREFIX_TDIM_N, "").parse::<usize>()? - 1;

        if let Value::String { value, comment } = value {
            Ok(Card::TableDimensionsN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid TDIMN data format".into())
        }
    }

    fn parse_table_column_format(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_TFORM_N)?;

        if let Value::String { value, comment } = value {
            Ok(Card::TableFormatN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid TFORMN data format".into())
        }
    }

    fn parse_table_display_format(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_TDISP_N)?;

        if let Value::String { value, comment } = value {
            Ok(Card::TableDisplayFormatN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid TDISPN data format".into())
        }
    }

    fn parse_table_null_value(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_TNULL_N)?;

        // Binary tables write an integer here and ASCII tables a string; the
        // card does not say which kind of table it belongs to, so accept both.
        let (value, comment) = match value {
            Value::Integer { value, comment } => (TableNullValue::Integer(value), comment),
            Value::String { value, comment } => (TableNullValue::Text(value), comment),
            _ => return Err("Invalid TNULLN data format".into()),
        };

        Ok(Card::TableNullValueN {
            index,
            value,
            comment,
        })
    }

    fn parse_table_scaling_factor(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_TSCAL_N)?;

        if let Some((value, comment)) = as_float(value) {
            Ok(Card::TableScalingFactorN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid TSCALN data format".into())
        }
    }

    fn parse_table_type(key: &str, buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_TTYPE_N)?;

        if let Value::String { value, comment } = value {
            Ok(Card::TableTypeN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid TTYPEN data format".into())
        }
    }

    fn parse_table_unit(key: &str, buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_TUNIT_N)?;

        if let Value::String { value, comment } = value {
            Ok(Card::TableUnitN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid TUNITN data format".into())
        }
    }

    fn parse_table_scaling_zero_point(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = parse_card_index(key, card_keys::PREFIX_TZERO_N)?;

        if let Some((value, comment)) = as_float(value) {
            Ok(Card::TableScalingZeroPointN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid TZERON data format".into())
        }
    }

    pub fn key(&self) -> String {
        match self {
            Card::Author { .. } => card_keys::AUTHOR.to_string(),
            Card::Bitpix { .. } => card_keys::BITPIX.to_string(),
            Card::Blank { .. } => card_keys::BLANK.to_string(),
            Card::Blocked { .. } => card_keys::BLOCKED.to_string(),
            Card::BScale { .. } => card_keys::BSCALE.to_string(),
            Card::BUnit { .. } => card_keys::BUNIT.to_string(),
            Card::BZero { .. } => card_keys::BZERO.to_string(),
            Card::Comment(_) => card_keys::COMMENT.to_string(),
            Card::DataMax { .. } => card_keys::DATAMAX.to_string(),
            Card::DataMin { .. } => card_keys::DATAMIN.to_string(),
            Card::Date { .. } => card_keys::DATE.to_string(),
            Card::DateObserved { .. } => card_keys::DATE_OBS.to_string(),
            Card::End => card_keys::END.to_string(),
            Card::Epoch { .. } => card_keys::EPOCH.to_string(),
            Card::Equinox { .. } => card_keys::EQUINOX.to_string(),
            Card::Extend { .. } => card_keys::EXTEND.to_string(),
            Card::ExtensionLevel { .. } => card_keys::EXTLEVEL.to_string(),
            Card::ExtensionName { .. } => card_keys::EXTNAME.to_string(),
            Card::ExtensionVersion { .. } => card_keys::EXTVER.to_string(),
            Card::GroupCount { .. } => card_keys::GCOUNT.to_string(),
            Card::Groups { .. } => card_keys::GROUPS.to_string(),
            Card::History(_) => card_keys::HISTORY.to_string(),
            Card::Instrument { .. } => card_keys::INSTRUME.to_string(),
            Card::NAxis { .. } => card_keys::NAXIS.to_string(),
            Card::Object { .. } => card_keys::OBJECT.to_string(),
            Card::Observer { .. } => card_keys::OBSERVER.to_string(),
            Card::Origin { .. } => card_keys::ORIGIN.to_string(),
            Card::ParameterCount { .. } => card_keys::PCOUNT.to_string(),
            Card::Reference { .. } => card_keys::REFERENC.to_string(),
            Card::Simple { .. } => card_keys::SIMPLE.to_string(),
            Card::Telescope { .. } => card_keys::TELESCOP.to_string(),
            Card::TableFields { .. } => card_keys::TFIELDS.to_string(),
            Card::TableHeap { .. } => card_keys::THEAP.to_string(),
            Card::Xtension { .. } => card_keys::XTENSION.to_string(),
            Card::FocalLength { .. } => card_keys::FOCALLEN.to_string(),
            Card::ExposureTime { .. } => card_keys::EXPTIME.to_string(),
            Card::CCDTemperature { .. } => card_keys::CCD_TEMP.to_string(),
            Card::BayerPattern { .. } => card_keys::BAYERPAT.to_string(),
            Card::Value { name, .. } => name.to_string(),
            Card::Continuation { .. } => "CONTINUE".to_string(),
            Card::Hierarch { name, .. } => name.to_string(),
            Card::Space => "".to_string(),
            Card::Undefined(_) => "".to_string(),
            Card::CoordinateDeltaN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_CDELT_N, index + 1)
            }
            Card::CoordinateRotationN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_CROTA_N, index + 1)
            }
            Card::CoordinateReferencePixelN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_CRPIX_N, index + 1)
            }
            Card::CoordinateValueAtPixelN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_CRVAL_N, index + 1)
            }
            Card::CoordinateAxisNameN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_CTYPE_N, index + 1)
            }
            Card::NAxisN { index, .. } => format!("{}{}", card_keys::PREFIX_NAXIS_N, index + 1),
            Card::ParameterScalingFactorN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_PSCAL_N, index + 1)
            }
            Card::ParameterTypeN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_PTYPE_N, index + 1)
            }
            Card::ParameterScalingZeroPointN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_PZERO_N, index + 1)
            }
            Card::TableColumnN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_TBCOL_N, index + 1)
            }
            Card::TableDimensionsN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_TDIM_N, index + 1)
            }
            Card::TableDisplayFormatN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_TDISP_N, index + 1)
            }
            Card::TableNullValueN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_TNULL_N, index + 1)
            }
            Card::TableScalingFactorN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_TSCAL_N, index + 1)
            }
            Card::TableTypeN { index, .. } => format!("{}{}", card_keys::PREFIX_TTYPE_N, index + 1),
            Card::TableUnitN { index, .. } => format!("{}{}", card_keys::PREFIX_TUNIT_N, index + 1),
            Card::TableScalingZeroPointN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_TZERO_N, index + 1)
            }
            Card::TableFormatN { index, .. } => {
                format!("{}{}", card_keys::PREFIX_TFORM_N, index + 1)
            }
            Card::Creator { .. } => card_keys::CREATOR.to_string(),
            Card::SubframeXPositionInBinnedPixels { .. } => card_keys::XORGSUBF.to_string(),
            Card::SubframeYPositionInBinnedPixels { .. } => card_keys::YORGSUBF.to_string(),
            Card::BinnedPixelsX { .. } => card_keys::XBINNING.to_string(),
            Card::BinnedPixelsY { .. } => card_keys::YBINNING.to_string(),
            Card::CCDBinnedPixelsX { .. } => card_keys::CCDXBIN.to_string(),
            Card::CCDBinnedPixelsY { .. } => card_keys::CCDYBIN.to_string(),
            Card::PixelSizeXWithBinningInMicrons { .. } => card_keys::XPIXSZ.to_string(),
            Card::PixelSizeYWithBinningInMicrons { .. } => card_keys::YPIXSZ.to_string(),
            Card::ImageType { .. } => card_keys::IMAGETYP.to_string(),
            Card::Exposure { .. } => card_keys::EXPOSURE.to_string(),
            Card::Ra { .. } => card_keys::RA.to_string(),
            Card::Dec { .. } => card_keys::DEC.to_string(),
            Card::GuideCam { .. } => card_keys::GUIDECAM.to_string(),
            Card::FocusPosition { .. } => card_keys::FOCUSPOS.to_string(),
            Card::SiteLongitude { .. } => card_keys::SITELONG.to_string(),
            Card::SiteLatitude { .. } => card_keys::SITELAT.to_string(),
            Card::ImageWidth { .. } => card_keys::IMAGEW.to_string(),
            Card::ImageHeight { .. } => card_keys::IMAGEH.to_string(),
        }
    }
}

fn parse_comment_text(buf: &[u8]) -> Result<String, Box<dyn Error + Send + Sync>> {
    let raw = std::str::from_utf8(buf.trim_ascii())?;
    Ok(raw.trim_ascii().into())
}

/// Parses the trailing 1-based index of an indexed keyword (`NAXIS3`, `TFORM7`, ...)
/// into the 0-based index used by [`Card`].
///
/// Returns an error rather than panicking on a malformed keyword: `NAXIS0` has no
/// 0-based equivalent, and `NAXISx` does not parse as a number at all.
fn parse_card_index(key: &str, prefix: &str) -> Result<usize, Box<dyn Error + Send + Sync>> {
    let index = key.replace(prefix, "");

    index
        .parse::<usize>()
        .map_err(|error| format!("Invalid index in card keyword {}: {}", key, error))?
        .checked_sub(1)
        .ok_or_else(|| format!("Card keyword {} is not a valid 1-based index", key).into())
}

/// Parses a CONTINUE card, which carries the next piece of a string too long
/// for one card.
///
/// A continuation has no `=`: the string simply starts after the keyword, and
/// the comment for the whole value may be attached to the last of them.
fn parse_continuation(buf: &[u8; 80]) -> Result<Card, Box<dyn Error + Send + Sync>> {
    let (text, comment) = split_value_and_comment(&buf[8..])?;

    let string = if text.starts_with('\'') {
        Some(parse_string(text)?)
    } else {
        None
    };

    Ok(Card::Continuation { string, comment })
}

/// Parses a HIERARCH card, which carries a keyword that the usual eight columns
/// cannot hold.
///
/// The name runs from after the keyword to the `=`, and is a series of words
/// which this keeps single-spaced so that the same keyword always reads the same
/// way.
fn parse_hierarch(buf: &[u8; 80]) -> Result<Card, Box<dyn Error + Send + Sync>> {
    let text = std::str::from_utf8(&buf[8..])?;

    let (name, value) = text
        .split_once('=')
        .ok_or("A HIERARCH card has no = separating its keyword from its value")?;

    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err("A HIERARCH card has no keyword".into());
    }

    Ok(Card::Hierarch {
        name,
        value: parse_value(value.as_bytes())?,
    })
}

fn parse_value(buf: &[u8]) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let (v, c) = split_value_and_comment(buf)?;
    if let Some(ch) = v.chars().next() {
        match ch {
            '\'' => Ok(Value::String {
                value: parse_string(v)?,
                comment: c,
            }),
            'T' => Ok(Value::Logical {
                value: true,
                comment: c,
            }),
            'F' => Ok(Value::Logical {
                value: false,
                comment: c,
            }),
            '(' => Err("Complex values not yet supported".into()),
            '0'..='9' | '-' | '+' | '.' => parse_number(v, c),
            _ => Ok(Value::Invalid(String::from_utf8_lossy(buf).into_owned())),
        }
    } else {
        Ok(Value::Undefined)
    }
}

fn parse_empty_keyword_card(buf: &[u8; 80]) -> Result<Card, Box<dyn Error + Send + Sync>> {
    let c = parse_comment_text(&buf[8..])?;
    if c.is_empty() {
        Ok(Card::Space)
    } else {
        Ok(Card::Comment(c))
    }
}

pub fn split_value_and_comment(
    buf: &[u8],
) -> Result<(String, Option<String>), Box<dyn Error + Send + Sync>> {
    let raw = std::str::from_utf8(buf.trim_ascii())?;

    if raw.starts_with("'") {
        let parts: Vec<_> = raw.split("'").collect();
        if let Some(part) = parts.get(2) {
            let comment_parts: Vec<_> = part.split("/").collect();
            return Ok((
                format!("'{}'", parts[1]).trim_ascii().into(),
                comment_parts
                    .get(1)
                    .map(|i| i.trim_ascii())
                    .map(|i| (*i).into()),
            ));
        }
    }

    let parts: Vec<_> = raw.split("/").collect();

    Ok((
        parts[0].trim_ascii().into(),
        parts.get(1).map(|i| i.trim_ascii()).map(|i| (*i).into()),
    ))
}

fn parse_string(s: String) -> Result<String, Box<dyn Error + Send + Sync>> {
    let start_quote = s.starts_with("'");
    let end_quote = s.ends_with("'");
    let value: &str = match (start_quote, end_quote) {
        (true, true) => &s[1..s.len() - 1], // string enclosed in single quotes
        (false, false) => &s,               // comment string has no quotes
        (true, false) => {
            return Err(format!("missing single quote at end, value was: {}", s).into());
        }
        (false, true) => {
            return Err(format!("missing single quote at start, value was: {}", s).into());
        }
    };

    if value.is_empty() {
        Ok(value.to_string())
    } else {
        let value = value.trim_end().to_string();
        if value.is_empty() {
            Ok("".to_string())
        } else {
            Ok(value)
        }
    }
}

/// Reads a card value that the FITS standard types as floating point.
///
/// A conforming writer may leave off the decimal point when the value happens
/// to be whole, and in practice most do: `BZERO = 32768` on unsigned 16-bit
/// images and `TSCAL1 = 1` on binary table columns are both spelled as
/// integers. Those parse as `Value::Integer`, so insisting on `Value::Float`
/// here rejected the card, and a rejected card fails the whole header.
fn as_float(value: Value) -> Option<(f64, Option<String>)> {
    match value {
        Value::Float { value, comment } => Some((value, comment)),
        Value::Integer { value, comment } => Some((value as f64, comment)),
        _ => None,
    }
}

fn parse_number(v: String, c: Option<String>) -> Result<Value, Box<dyn Error + Send + Sync>> {
    if v.is_empty() {
        Ok(Value::Undefined) // FITSv4, section 4.1.2.3
    } else if let Ok(val) = v.parse::<i64>() {
        // First parse integer
        Ok(Value::Integer {
            value: val,
            comment: c,
        })
    } else if let Ok(val) = v.parse::<f64>() {
        // If it fails try parsing a float
        Ok(Value::Float {
            value: val,
            comment: c,
        })
    } else {
        // fallback to D as an exponent, cf. FITSv4, section 4.2.4 Real floating-point number
        let v = v.replace('D', "E");
        if let Ok(val) = v.parse::<f64>() {
            Ok(Value::Float {
                value: val,
                comment: c,
            })
        } else {
            Ok(Value::Invalid(v))
        }
    }
}

/// Column, counting from 0, at which a fixed-format value ends.
///
/// The FITS standard puts the value in columns 11 to 30 and right-justifies
/// numbers and logicals against column 30.
const VALUE_END_COLUMN: usize = 30;

impl Card {
    /// Renders this card as the 80 bytes it occupies in a header.
    ///
    /// Cards are written in the standard's fixed format: the keyword in columns
    /// 1 to 8, `= ` in columns 9 and 10, and the value from column 11 — numbers
    /// and logicals right-justified against column 30, strings quoted and
    /// left-justified — followed by ` / ` and the comment if there is room for
    /// one.
    pub fn to_bytes(&self) -> [u8; CARD_NUM_BYTES] {
        let text = match self {
            Card::End => card_keys::END.to_string(),
            Card::Space => String::new(),

            // COMMENT and HISTORY carry free text rather than a value, so they
            // have no `= ` and run from column 9 to the end of the card.
            Card::Comment(text) => format!("{:<8}{}", card_keys::COMMENT, text),
            Card::History(text) => format!("{:<8}{}", card_keys::HISTORY, text),

            // A card this crate could not interpret is written back exactly as
            // it was read, so that reading and writing a file leaves it alone.
            Card::Undefined(text) => text.clone(),

            Card::Hierarch { name, value } => {
                format!("HIERARCH {} = {}", name, unquoted(value))
            }

            // A continuation is normally written by the card it belongs to;
            // one standing on its own is written back as it was read.
            Card::Continuation { string, comment } => with_comment(
                format!("CONTINUE  '{}'", string.clone().unwrap_or_default()),
                comment.as_deref(),
            ),

            card => format_value_card(&card.key(), &Value::from(card)),
        };

        let mut bytes = [b' '; CARD_NUM_BYTES];
        for (slot, byte) in bytes.iter_mut().zip(text.bytes()) {
            *slot = byte;
        }
        bytes
    }

    /// The string this card holds, so that a continuation can be appended to
    /// it.
    ///
    /// Only the string-valued cards can be continued, which is what the long
    /// value convention is for. A keyword this does not know about simply will
    /// not join its continuations, rather than joining them wrongly.
    pub(crate) fn string_value_mut(&mut self) -> Option<&mut String> {
        match self {
            Card::Author { value, .. }
            | Card::BUnit { value, .. }
            | Card::CoordinateAxisNameN { value, .. }
            | Card::Creator { value, .. }
            | Card::ExtensionName { value, .. }
            | Card::GuideCam { value, .. }
            | Card::Instrument { value, .. }
            | Card::Object { value, .. }
            | Card::Observer { value, .. }
            | Card::Origin { value, .. }
            | Card::ParameterTypeN { value, .. }
            | Card::Reference { value, .. }
            | Card::TableDimensionsN { value, .. }
            | Card::TableDisplayFormatN { value, .. }
            | Card::TableFormatN { value, .. }
            | Card::TableTypeN { value, .. }
            | Card::TableUnitN { value, .. }
            | Card::Telescope { value, .. } => Some(value),

            Card::Value {
                value: Value::String { value, .. },
                ..
            }
            | Card::Hierarch {
                value: Value::String { value, .. },
                ..
            } => Some(value),

            _ => None,
        }
    }

    /// Replaces this card's comment, for the string cards a continuation can
    /// carry one for.
    pub(crate) fn set_comment(&mut self, text: String) {
        let slot = match self {
            Card::Author { comment, .. }
            | Card::BUnit { comment, .. }
            | Card::CoordinateAxisNameN { comment, .. }
            | Card::Creator { comment, .. }
            | Card::ExtensionName { comment, .. }
            | Card::GuideCam { comment, .. }
            | Card::Instrument { comment, .. }
            | Card::Object { comment, .. }
            | Card::Observer { comment, .. }
            | Card::Origin { comment, .. }
            | Card::ParameterTypeN { comment, .. }
            | Card::Reference { comment, .. }
            | Card::TableDimensionsN { comment, .. }
            | Card::TableDisplayFormatN { comment, .. }
            | Card::TableFormatN { comment, .. }
            | Card::TableTypeN { comment, .. }
            | Card::TableUnitN { comment, .. }
            | Card::Telescope { comment, .. } => comment,

            Card::Value {
                value: Value::String { comment, .. },
                ..
            }
            | Card::Hierarch {
                value: Value::String { comment, .. },
                ..
            } => comment,

            _ => return,
        };

        *slot = Some(text);
    }

    /// Renders this card as the cards it occupies in a header.
    ///
    /// Almost every card is one card. A string value too long to fit, or one
    /// whose comment will not fit beside it, is written as a first card ending
    /// in `&` followed by CONTINUE cards — the convention FITS uses for values
    /// longer than a card can hold.
    pub fn to_cards(&self) -> Vec<[u8; CARD_NUM_BYTES]> {
        // A HIERARCH card is laid out by hand, and a continuation belongs to the
        // card that produced it.
        if matches!(self, Card::Hierarch { .. } | Card::Continuation { .. }) {
            return vec![self.to_bytes()];
        }

        let Value::String { value, comment } = Value::from(self) else {
            return vec![self.to_bytes()];
        };

        let escaped = value.replace('\'', "''");

        // `KEYWORD = '` and the closing quote account for twelve columns.
        let room = CARD_NUM_BYTES - 12;
        let comment_room = comment.as_ref().map(|comment| comment.len() + 3);

        if escaped.len() <= room
            && comment_room
                .is_none_or(|needed| escaped.len().max(8) + 12 + needed <= CARD_NUM_BYTES)
        {
            return vec![self.to_bytes()];
        }

        // Every card but the last needs a column for the `&` that says more
        // follows.
        let mut chunks = chunk(&escaped, room - 1);

        // A comment with nowhere to sit gets a continuation of its own.
        if let Some(needed) = comment_room {
            let last = chunks.last().map(String::len).unwrap_or(0);
            if last + 12 + needed > CARD_NUM_BYTES {
                chunks.push(String::new());
            }
        }

        let last = chunks.len() - 1;

        chunks
            .into_iter()
            .enumerate()
            .map(|(index, text)| {
                let more = index < last;
                let comment = (index == last).then_some(comment.as_deref()).flatten();

                pad(&if index == 0 {
                    with_comment(
                        format!("{:<8}= '{}{}'", self.key(), text, mark(more)),
                        comment,
                    )
                } else {
                    with_comment(format!("CONTINUE  '{}{}'", text, mark(more)), comment)
                })
            })
            .collect()
    }
}

/// Splits `text` into pieces of at most `room` bytes, on character boundaries.
fn chunk(text: &str, room: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut rest = text;

    while !rest.is_empty() {
        let mut at = room.min(rest.len());
        while at > 0 && !rest.is_char_boundary(at) {
            at -= 1;
        }

        let (head, tail) = rest.split_at(at.max(1).min(rest.len()));
        chunks.push(head.to_string());
        rest = tail;
    }

    if chunks.is_empty() {
        chunks.push(String::new());
    }

    chunks
}

fn mark(more: bool) -> &'static str {
    if more { "&" } else { "" }
}

/// Adds a comment to a card if there is room for it.
fn with_comment(card: String, comment: Option<&str>) -> String {
    match comment {
        Some(comment) if card.len() + 3 + comment.len() <= CARD_NUM_BYTES => {
            format!("{} / {}", card, comment)
        }
        _ => card,
    }
}

fn pad(text: &str) -> [u8; CARD_NUM_BYTES] {
    let mut bytes = [b' '; CARD_NUM_BYTES];
    for (slot, byte) in bytes.iter_mut().zip(text.bytes()) {
        *slot = byte;
    }
    bytes
}

/// A value as it appears after the `= ` of a HIERARCH card, which is not laid
/// out in the fixed columns.
fn unquoted(value: &Value) -> String {
    match value {
        Value::String { value, .. } => format!("'{}'", value.replace('\'', "''")),
        other => other.value_to_string(),
    }
}

/// Renders a keyword and its value in the standard's fixed format.
fn format_value_card(key: &str, value: &Value) -> String {
    let rendered = match value {
        // A string is quoted, with any embedded quote doubled, and padded to
        // the eight characters the standard asks for as a minimum.
        Value::String { value, .. } => {
            let escaped = value.replace('\'', "''");
            format!("'{:<8}'", escaped)
        }
        Value::Logical { .. } | Value::Integer { .. } | Value::Float { .. } => {
            format!("{:>20}", value.value_to_string())
        }
        // An undefined value is written as an empty field, which is how the
        // standard spells "this keyword has no value".
        Value::Undefined | Value::Invalid(_) => " ".repeat(20),
    };

    let mut card = format!("{:<8}= {}", key, rendered);

    // The comment is optional, and dropped rather than truncated when the value
    // has already used the card up.
    if let Some(comment) = comment_of(value) {
        let padded = format!("{:<width$}", card, width = VALUE_END_COLUMN);
        let with_comment = format!("{} / {}", padded, comment);
        if with_comment.len() <= CARD_NUM_BYTES {
            return with_comment;
        }
        card = padded;
    }

    card
}

fn comment_of(value: &Value) -> Option<&str> {
    match value {
        Value::Integer { comment, .. }
        | Value::Float { comment, .. }
        | Value::Logical { comment, .. }
        | Value::String { comment, .. } => comment.as_deref(),
        Value::Undefined | Value::Invalid(_) => None,
    }
}

#[cfg(test)]
mod write_tests {
    use super::Card;
    use crate::header::card_keys;

    fn rendered(card: &Card) -> String {
        String::from_utf8(card.to_bytes().to_vec()).expect("cards are ASCII")
    }

    #[test]
    fn every_card_is_exactly_eighty_bytes() {
        for card in [
            Card::End,
            Card::Space,
            Card::Comment("a comment".into()),
            Card::Simple {
                value: true,
                comment: None,
            },
            Card::ExtensionName {
                value: "a very long extension name indeed".into(),
                comment: Some("with a comment that will not fit alongside it".into()),
            },
        ] {
            assert_eq!(card.to_bytes().len(), 80, "{card:?}");
            assert_eq!(rendered(&card).len(), 80, "{card:?}");
        }
    }

    #[test]
    fn a_logical_is_right_justified_against_column_thirty() {
        let card = rendered(&Card::Simple {
            value: true,
            comment: None,
        });

        assert!(
            card.starts_with("SIMPLE  =                    T"),
            "{card:?}"
        );
        assert_eq!(&card[29..30], "T");
    }

    #[test]
    fn an_integer_is_right_justified_and_keeps_its_comment() {
        let card = rendered(&Card::NAxis {
            value: 2,
            comment: Some("number of axes".into()),
        });

        assert!(
            card.starts_with("NAXIS   =                    2 / number of axes"),
            "{card:?}"
        );
    }

    #[test]
    fn a_string_is_quoted_and_padded_to_eight_characters() {
        let card = rendered(&Card::ExtensionName {
            value: "SCI".into(),
            comment: None,
        });

        assert!(card.starts_with("EXTNAME = 'SCI     '"), "{card:?}");
    }

    #[test]
    fn a_quote_inside_a_string_is_doubled() {
        // The standard escapes a single quote by writing it twice, and a reader
        // that met a lone quote would end the string early.
        let card = rendered(&Card::Object {
            value: "Barnard's Star".into(),
            comment: None,
        });

        assert!(card.contains("'Barnard''s Star'"), "{card:?}");
    }

    #[test]
    fn a_comment_that_does_not_fit_is_dropped_rather_than_overflowing() {
        let card = rendered(&Card::Object {
            value: "a".repeat(60),
            comment: Some("this cannot possibly fit".into()),
        });

        assert_eq!(card.len(), 80);
        assert!(!card.contains("this cannot"), "{card:?}");
    }

    #[test]
    fn comment_and_history_cards_carry_free_text() {
        let card = rendered(&Card::Comment("processed by fits-io".into()));
        assert!(card.starts_with("COMMENT processed by fits-io"), "{card:?}");

        let card = rendered(&Card::History("resampled".into()));
        assert!(card.starts_with("HISTORY resampled"), "{card:?}");
    }

    #[test]
    fn the_end_card_is_the_keyword_and_nothing_else() {
        let card = rendered(&Card::End);

        assert!(card.starts_with(card_keys::END));
        assert_eq!(card.trim_end(), card_keys::END);
    }

    #[test]
    fn a_card_round_trips_through_bytes() {
        for original in [
            Card::Simple {
                value: true,
                comment: None,
            },
            Card::NAxis {
                value: 3,
                comment: Some("axes".into()),
            },
            Card::ExtensionName {
                value: "SCI".into(),
                comment: None,
            },
        ] {
            let bytes = original.to_bytes();
            let parsed = Card::try_from(&bytes).expect("a card this crate wrote must parse");

            assert_eq!(
                parsed,
                original,
                "from {:?}",
                String::from_utf8_lossy(&bytes)
            );
        }
    }
}
