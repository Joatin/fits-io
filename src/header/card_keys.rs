////////////////////
// OFFICIAL CARDS //
////////////////////

/// Author of the data
pub const AUTHOR: &str = "AUTHOR";

/// bits per data value
pub const BITPIX: &str = "BITPIX";

/// value used for undefined array elements
pub const BLANK: &str = "BLANK";

/// Deprecated since version 1 of the standard
pub const BLOCKED: &str = "BLOCKED";

/// linear factor in scaling equation
pub const BSCALE: &str = "BSCALE";

/// physical units of the array values
pub const BUNIT: &str = "BUNIT";

/// zero point in scaling equation
pub const BZERO: &str = "BZERO";

/// coordinate increment along axis
pub const PREFIX_CDELT_N: &str = "CDELT";

/// descriptive comment
pub const COMMENT: &str = "COMMENT";

/// coordinate system rotation angle
pub const PREFIX_CROTA_N: &str = "CROTA";

/// coordinate system reference pixel
pub const PREFIX_CRPIX_N: &str = "CRPIX";

/// coordinate system value at reference pixel
pub const PREFIX_CRVAL_N: &str = "CRVAL";

/// name of the coordinate axis
pub const PREFIX_CTYPE_N: &str = "CTYPE";

/// maximum data value
pub const DATAMAX: &str = "DATAMAX";

/// minimum data value
pub const DATAMIN: &str = "DATAMIN";

/// date of file creation
pub const DATE: &str = "DATE";

/// date of the observation
pub const DATE_OBS: &str = "DATE-OBS";

/// marks the end of the header keywords
pub const END: &str = "END";

/// equinox of celestial coordinate system
pub const EPOCH: &str = "EPOCH";

/// equinox of celestial coordinate system
pub const EQUINOX: &str = "EQUINOX";

/// may the FITS file contain extensions?
pub const EXTEND: &str = "EXTEND";

/// hierarchical level of the extension
pub const EXTLEVEL: &str = "EXTLEVEL";

/// name of the extension
pub const EXTNAME: &str = "EXTNAME";

/// version of the extension
pub const EXTVER: &str = "EXTVER";

/// group count
pub const GCOUNT: &str = "GCOUNT";

/// indicates random groups structure
pub const GROUPS: &str = "GROUPS";

/// processing history of the data
pub const HISTORY: &str = "HISTORY";

/// name of instrument
pub const INSTRUME: &str = "INSTRUME";

/// number of axes
pub const NAXIS: &str = "NAXIS";

pub const PREFIX_NAXIS_N: &str = "NAXIS";

/// name of observed object
pub const OBJECT: &str = "OBJECT";

/// observer who acquired the data
pub const OBSERVER: &str = "OBSERVER";

/// organization responsible for the data
pub const ORIGIN: &str = "ORIGIN";

/// parameter count
pub const PCOUNT: &str = "PCOUNT";

/// parameter scaling factor
pub const PREFIX_PSCAL_N: &str = "PSCAL";

/// name of random groups parameter
pub const PREFIX_PTYPE_N: &str = "PTYPE";

/// parameter scaling zero point
pub const PREFIX_PZERO_N: &str = "PZERO";

/// bibliographic reference
pub const REFERENC: &str = "REFERENC";

/// does file conform to the Standard?
pub const SIMPLE: &str = "SIMPLE";

/// begining column number
pub const PREFIX_TBCOL_N: &str = "TBCOL";

/// dimensionality of the array
pub const PREFIX_TDIM_N: &str = "TDIM";

/// display format
pub const PREFIX_TDISP_N: &str = "TDISP";

/// name of telescope
pub const TELESCOP: &str = "TELESCOP";

/// number of columns in the table
pub const TFIELDS: &str = "TFIELDS";

/// column data format
pub const PREFIX_TFORM_N: &str = "TFORM";

/// offset to starting data heap address
pub const THEAP: &str = "THEAP";

/// value used to indicate undefined table element
pub const PREFIX_TNULL_N: &str = "TNULL";

/// linear data scaling factor
pub const PREFIX_TSCAL_N: &str = "TSCAL";

/// column name
pub const PREFIX_TTYPE_N: &str = "TTYPE";

/// column units
pub const PREFIX_TUNIT_N: &str = "TUNIT";

/// column scaling zero point
pub const PREFIX_TZERO_N: &str = "TZERO";

/// marks beginning of new HDU
pub const XTENSION: &str = "XTENSION";

//////////////////////
// ADDITIONAL CARDS //
//////////////////////

pub const FOCALLEN: &str = "FOCALLEN";
pub const EXPTIME: &str = "EXPTIME";
pub const CCD_TEMP: &str = "CCD-TEMP";
pub const BAYERPAT: &str = "BAYERPAT";
pub const CREATOR: &str = "CREATOR";
pub const XORGSUBF: &str = "XORGSUBF";
pub const YORGSUBF: &str = "YORGSUBF";
pub const XBINNING: &str = "XBINNING";
pub const YBINNING: &str = "YBINNING";
pub const CCDXBIN: &str = "CCDXBIN";
pub const CCDYBIN: &str = "CCDYBIN";
pub const XPIXSZ: &str = "XPIXSZ";
pub const YPIXSZ: &str = "YPIXSZ";
pub const IMAGETYP: &str = "IMAGETYP";
pub const EXPOSURE: &str = "EXPOSURE";
pub const RA: &str = "RA";
pub const DEC: &str = "DEC";
pub const GUIDECAM: &str = "GUIDECAM";
pub const FOCUSPOS: &str = "FOCUSPOS";
pub const SITELONG: &str = "SITELONG";
pub const SITELAT: &str = "SITELAT";
pub const IMAGEW: &str = "IMAGEW";
pub const IMAGEH: &str = "IMAGEH";
