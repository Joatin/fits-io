use crate::header::extension_type::ExtensionType;
use crate::header::value::Value;
use crate::header::{BayerPattern, Bitpix, ImageType, TableColumnFormat, card_keys};
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use chrono::{DateTime, NaiveDateTime, Utc};
use core::error::Error;

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
        value: String,
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
    TableFormatN {
        index: usize,
        value: TableColumnFormat,
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
        value: core::time::Duration,
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
        value: core::time::Duration,
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
        if let Value::Float { value, comment } = value {
            Ok(Card::BScale { value, comment })
        } else if let Value::Integer { value, comment } = value {
            // Some use wrong datatype for this value, lets handle it
            Ok(Card::BScale {
                value: value as f64,
                comment,
            })
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
        if let Value::Float { value, comment } = value {
            Ok(Card::BZero { value, comment })
        } else if let Value::Integer { value, comment } = value {
            // Some use wrong datatype for this value, lets handle it
            Ok(Card::BZero {
                value: value as f64,
                comment,
            })
        } else {
            Err("Invalid bzero data format".into())
        }
    }

    fn parse_data_max(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Float { value, comment } = value {
            Ok(Card::DataMax { value, comment })
        } else {
            Err("Invalid data max data format".into())
        }
    }

    fn parse_data_min(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Float { value, comment } = value {
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
        if let Value::Float { value, comment } = value {
            Ok(Card::Epoch { value, comment })
        } else {
            Err("Invalid epoch data format".into())
        }
    }

    fn parse_equinox(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Float { value, comment } = value {
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
        if let Value::Float { value, comment } = value {
            Ok(Card::FocalLength { value, comment })
        } else if let Value::Integer { value, comment } = value {
            Ok(Card::FocalLength {
                value: value as f64,
                comment,
            })
        } else {
            Err("Invalid focallen data format".into())
        }
    }

    fn parse_exposure_time(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Float { value, comment } = value {
            let value = std::time::Duration::from_secs_f64(value);
            Ok(Card::ExposureTime { value, comment })
        } else {
            Err("Invalid exptime data format".into())
        }
    }

    fn parse_ccd_temperature(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Float { value, comment } = value {
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
        if let Value::Float { value, comment } = value {
            Ok(Card::PixelSizeXWithBinningInMicrons { value, comment })
        } else {
            Err("Invalid XPIXSZ data format".into())
        }
    }

    fn parse_pixel_size_y_with_binning_in_microns(
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Float { value, comment } = value {
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
        if let Value::Float { value, comment } = value {
            let value = std::time::Duration::from_secs_f64(value);
            Ok(Card::Exposure { value, comment })
        } else {
            Err("Invalid EXPOSURE data format".into())
        }
    }

    fn parse_ra(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Float { value, comment } = value {
            Ok(Card::Ra { value, comment })
        } else {
            Err("Invalid RA data format".into())
        }
    }

    fn parse_dec(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Float { value, comment } = value {
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
        if let Value::Float { value, comment } = value {
            Ok(Card::SiteLongitude { value, comment })
        } else {
            Err("Invalid SITELONG data format".into())
        }
    }

    fn parse_site_latitude(buf: &[u8; 80]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;
        if let Value::Float { value, comment } = value {
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

        let index = key
            .replace(card_keys::PREFIX_CDELT_N, "")
            .parse::<usize>()?
            - 1;

        if let Value::Float { value, comment } = value {
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

        let index = key
            .replace(card_keys::PREFIX_CROTA_N, "")
            .parse::<usize>()?
            - 1;

        if let Value::Float { value, comment } = value {
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

        let index = key
            .replace(card_keys::PREFIX_CRPIX_N, "")
            .parse::<usize>()?
            - 1;

        if let Value::Float { value, comment } = value {
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

        let index = key
            .replace(card_keys::PREFIX_CRVAL_N, "")
            .parse::<usize>()?
            - 1;

        if let Value::Float { value, comment } = value {
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

        let index = key
            .replace(card_keys::PREFIX_CTYPE_N, "")
            .parse::<usize>()?
            - 1;

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

        let index = key
            .replace(card_keys::PREFIX_NAXIS_N, "")
            .parse::<usize>()?
            - 1;

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

        let index = key
            .replace(card_keys::PREFIX_PSCAL_N, "")
            .parse::<usize>()?
            - 1;

        if let Value::Float { value, comment } = value {
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

        let index = key
            .replace(card_keys::PREFIX_PTYPE_N, "")
            .parse::<usize>()?
            - 1;

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

        let index = key
            .replace(card_keys::PREFIX_PZERO_N, "")
            .parse::<usize>()?
            - 1;

        if let Value::Float { value, comment } = value {
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

        let index = key
            .replace(card_keys::PREFIX_TBCOL_N, "")
            .parse::<usize>()?
            - 1;

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

        let index = key
            .replace(card_keys::PREFIX_TFORM_N, "")
            .parse::<usize>()?
            - 1;

        if let Value::String { value, comment } = value {
            Ok(Card::TableFormatN {
                index,
                value: value.try_into()?,
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

        let index = key
            .replace(card_keys::PREFIX_TDISP_N, "")
            .parse::<usize>()?
            - 1;

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

        let index = key
            .replace(card_keys::PREFIX_TNULL_N, "")
            .parse::<usize>()?
            - 1;

        if let Value::String { value, comment } = value {
            Ok(Card::TableNullValueN {
                index,
                value,
                comment,
            })
        } else {
            Err("Invalid TNULLN data format".into())
        }
    }

    fn parse_table_scaling_factor(
        key: &str,
        buf: &[u8; 80],
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let value = parse_value(&buf[10..])?;

        let index = key
            .replace(card_keys::PREFIX_TSCAL_N, "")
            .parse::<usize>()?
            - 1;

        if let Value::Float { value, comment } = value {
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

        let index = key
            .replace(card_keys::PREFIX_TTYPE_N, "")
            .parse::<usize>()?
            - 1;

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

        let index = key
            .replace(card_keys::PREFIX_TUNIT_N, "")
            .parse::<usize>()?
            - 1;

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

        let index = key
            .replace(card_keys::PREFIX_TZERO_N, "")
            .parse::<usize>()?
            - 1;

        if let Value::Float { value, comment } = value {
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
            Card::Hierarch { .. } => "HIERARCH".to_string(),
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

fn parse_continuation(_buf: &[u8; 80]) -> Result<Card, Box<dyn Error + Send + Sync>> {
    Ok(Card::Undefined("".into()))
}

fn parse_hierarch(_buf: &[u8; 80]) -> Result<Card, Box<dyn Error + Send + Sync>> {
    Ok(Card::Undefined("".into()))
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
