use std::error::Error;

/// How a flat image plane is carried onto the celestial sphere.
///
/// The projection is named by the last three characters of CTYPEn, after the
/// coordinate type and a hyphen — `RA---TAN` is right ascension under the
/// gnomonic projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    /// No projection code: the intermediate coordinates *are* the world
    /// coordinates, offset from the reference value. This is what a plain
    /// `LINEAR`, `PIXEL` or wavelength axis uses.
    Linear,
    /// `TAN`, the gnomonic projection. Straight lines on the sky stay straight
    /// on the plane, which is what an ordinary telescope with a flat detector
    /// produces, and by far the most common projection in astronomy images.
    Gnomonic,
}

impl Projection {
    /// Reads the projection out of a CTYPEn card.
    ///
    /// # Errors
    ///
    /// Returns an error for a projection this crate does not implement, rather
    /// than silently falling back to a linear mapping — a wrong projection
    /// yields plausible coordinates that are quietly in the wrong place.
    pub fn from_ctype(ctype: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let ctype = ctype.trim();

        // The projection code is the part after the last hyphen, and a CTYPEn
        // with no hyphen names no projection at all.
        let Some((_, code)) = ctype.rsplit_once('-') else {
            return Ok(Projection::Linear);
        };

        match code {
            "" | "LINEAR" | "PIXEL" => Ok(Projection::Linear),
            "TAN" => Ok(Projection::Gnomonic),
            other => Err(From::from(format!(
                "Unsupported WCS projection in CTYPE {:?}: {} is not implemented",
                ctype, other
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Projection;

    #[test]
    fn a_ctype_names_its_projection_in_its_last_field() {
        assert_eq!(
            Projection::from_ctype("RA---TAN").unwrap(),
            Projection::Gnomonic
        );
        assert_eq!(
            Projection::from_ctype("DEC--TAN").unwrap(),
            Projection::Gnomonic
        );
        assert_eq!(
            Projection::from_ctype("LINEAR").unwrap(),
            Projection::Linear
        );
    }

    #[test]
    fn an_unimplemented_projection_is_an_error_rather_than_a_wrong_answer() {
        // Falling back to a linear mapping here would hand back coordinates that
        // look entirely reasonable and are in the wrong place.
        let error =
            Projection::from_ctype("RA---SIN").expect_err("the SIN projection is not implemented");

        assert!(error.to_string().contains("SIN"), "got: {error}");
    }
}
