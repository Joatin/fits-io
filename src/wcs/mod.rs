//! Turning pixel positions into sky coordinates, and back.
//!
//! A FITS header describes this mapping with the WCS keywords: CRPIXn says which
//! pixel is the reference point, CRVALn what sky coordinate that pixel sits at,
//! CDELTn the scale, CROTA2 the rotation, and CTYPEn which projection carries
//! the plane onto the sphere.

mod distortion;
mod projection;
mod spherical;
#[allow(clippy::module_inception)]
mod wcs;

pub use self::projection::Projection;
pub use self::wcs::Wcs;
