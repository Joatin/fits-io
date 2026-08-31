use std::error::Error;

/// How a flat image plane is carried onto the celestial sphere.
///
/// The projection is named by the last three characters of CTYPEn, after the
/// coordinate type and a hyphen — `RA---TAN` is right ascension under the
/// gnomonic projection.
///
/// Each projection converts between the intermediate world coordinates the
/// header's matrix produces and the *native* spherical coordinates of the
/// projection's own frame; [`Wcs`] then rotates those onto the sky. A point the
/// projection cannot represent — the far hemisphere under TAN, say — comes back
/// as `NaN` rather than as a plausible coordinate in the wrong place.
///
/// [`Wcs`]: crate::wcs::Wcs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Projection {
    /// No projection code: the intermediate coordinates *are* the world
    /// coordinates, offset from the reference value. This is what a plain
    /// `LINEAR`, `PIXEL` or wavelength axis uses.
    Linear,
    /// `TAN`, the gnomonic projection. Straight lines on the sky stay straight
    /// on the plane, which is what an ordinary telescope with a flat detector
    /// produces, and by far the most common projection in astronomy images.
    Gnomonic,
    /// `SIN`, the orthographic projection: the sphere as seen from infinitely
    /// far away. Radio interferometers image in it.
    Orthographic,
    /// `STG`, the stereographic projection, which preserves angles.
    Stereographic,
    /// `ARC`, the zenithal equidistant projection, where radius from the
    /// reference point is the angle from it. Schmidt plates and many all-sky
    /// cameras use it.
    ZenithalEquidistant,
    /// `ZEA`, the zenithal equal-area projection, which preserves area.
    ZenithalEqualArea,
    /// `CAR`, the plate carrée: longitude and latitude used directly as
    /// rectangular coordinates.
    PlateCarree,
    /// `MER`, the Mercator projection.
    Mercator,
    /// `CEA`, the cylindrical equal-area projection, whose standard parallel is
    /// set by `PV2_1`.
    CylindricalEqualArea,
    /// `AIT`, the Hammer-Aitoff projection, an equal-area whole-sky projection.
    /// All-sky maps are usually drawn in it.
    HammerAitoff,
    /// `MOL`, the Mollweide projection, another equal-area whole-sky one.
    Mollweide,
}

/// The parameters some projections take from the header's `PVi_j` cards.
///
/// Only a few projections need one, and the rest ignore what is here.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct ProjectionParams {
    /// `PV2_1`: the standard parallel of `CEA`, in the units the projection
    /// defines for it.
    pub cea_lambda: Option<f64>,
}

/// How far the tangent plane is from the sphere's centre, in degrees per radian.
const DEGREES_PER_RADIAN: f64 = 180.0 / std::f64::consts::PI;

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
            "TAN" | "TPV" => Ok(Projection::Gnomonic),
            "SIN" => Ok(Projection::Orthographic),
            "STG" => Ok(Projection::Stereographic),
            "ARC" => Ok(Projection::ZenithalEquidistant),
            "ZEA" => Ok(Projection::ZenithalEqualArea),
            "CAR" => Ok(Projection::PlateCarree),
            "MER" => Ok(Projection::Mercator),
            "CEA" => Ok(Projection::CylindricalEqualArea),
            "AIT" => Ok(Projection::HammerAitoff),
            "MOL" => Ok(Projection::Mollweide),
            other => Err(From::from(format!(
                "Unsupported WCS projection in CTYPE {:?}: {} is not implemented",
                ctype, other
            ))),
        }
    }

    /// The projection's three-letter code, or `LINEAR` for no projection at all.
    pub fn code(self) -> &'static str {
        match self {
            Projection::Linear => "LINEAR",
            Projection::Gnomonic => "TAN",
            Projection::Orthographic => "SIN",
            Projection::Stereographic => "STG",
            Projection::ZenithalEquidistant => "ARC",
            Projection::ZenithalEqualArea => "ZEA",
            Projection::PlateCarree => "CAR",
            Projection::Mercator => "MER",
            Projection::CylindricalEqualArea => "CEA",
            Projection::HammerAitoff => "AIT",
            Projection::Mollweide => "MOL",
        }
    }

    /// The native coordinates of the point the reference pixel sits at, in
    /// degrees.
    ///
    /// The zenithal projections are drawn about their pole, so their reference
    /// point is the native pole; every other projection here is drawn about its
    /// origin.
    pub(crate) fn fiducial(self) -> (f64, f64) {
        if self.is_zenithal() {
            (0.0, 90.0)
        } else {
            (0.0, 0.0)
        }
    }

    /// Whether this projection is drawn about a pole rather than an origin.
    fn is_zenithal(self) -> bool {
        matches!(
            self,
            Projection::Gnomonic
                | Projection::Orthographic
                | Projection::Stereographic
                | Projection::ZenithalEquidistant
                | Projection::ZenithalEqualArea
        )
    }

    /// The native spherical coordinates `(phi, theta)` of a point at the
    /// intermediate world coordinates `(x, y)`, all in degrees.
    ///
    /// A point outside what the projection can represent comes back as `NaN`.
    pub(crate) fn to_native(self, x: f64, y: f64, params: &ProjectionParams) -> (f64, f64) {
        if self.is_zenithal() {
            let radius = x.hypot(y);
            // Native longitude is measured from the negative y axis, which is
            // what puts north up in an image with no rotation.
            let phi = if radius == 0.0 {
                0.0
            } else {
                x.atan2(-y).to_degrees()
            };

            return (phi, self.zenithal_theta(radius));
        }

        match self {
            Projection::PlateCarree => (x, y),
            Projection::Mercator => (
                x,
                2.0 * (y / DEGREES_PER_RADIAN).exp().atan().to_degrees() - 90.0,
            ),
            Projection::CylindricalEqualArea => {
                let lambda = params.cea_lambda.unwrap_or(1.0);
                let sine = lambda * y / DEGREES_PER_RADIAN;
                (x, arcsine(sine))
            }
            Projection::HammerAitoff => {
                let (x, y) = (x / DEGREES_PER_RADIAN, y / DEGREES_PER_RADIAN);
                let inside = 1.0 - (x / 4.0).powi(2) - (y / 2.0).powi(2);

                if inside < 0.0 {
                    return (f64::NAN, f64::NAN);
                }

                let z = inside.sqrt();
                let phi = 2.0 * (z * x / 2.0).atan2(2.0 * z * z - 1.0);

                (phi.to_degrees(), arcsine(y * z))
            }
            Projection::Mollweide => {
                let (x, y) = (x / DEGREES_PER_RADIAN, y / DEGREES_PER_RADIAN);
                let root_two = std::f64::consts::SQRT_2;

                let sine = y / root_two;
                if !(-1.0..=1.0).contains(&sine) {
                    return (f64::NAN, f64::NAN);
                }

                let gamma = sine.asin();
                let cosine = gamma.cos();
                if cosine == 0.0 {
                    // The poles, where every longitude meets.
                    return (0.0, 90.0_f64.copysign(y));
                }

                let phi = std::f64::consts::PI * x / (2.0 * root_two * cosine);
                let theta = arcsine((2.0 * gamma + (2.0 * gamma).sin()) / std::f64::consts::PI);

                (phi.to_degrees(), theta)
            }
            // The zenithal projections are handled above, and a linear axis
            // never reaches here.
            Projection::Linear
            | Projection::Gnomonic
            | Projection::Orthographic
            | Projection::Stereographic
            | Projection::ZenithalEquidistant
            | Projection::ZenithalEqualArea => (x, y),
        }
    }

    /// The intermediate world coordinates of a native spherical position, the
    /// inverse of [`Projection::to_native`].
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn from_native(self, phi: f64, theta: f64, params: &ProjectionParams) -> (f64, f64) {
        if self.is_zenithal() {
            let radius = self.zenithal_radius(theta);
            let (sin, cos) = phi.to_radians().sin_cos();

            return (radius * sin, -radius * cos);
        }

        match self {
            Projection::PlateCarree => (phi, theta),
            Projection::Mercator => {
                let half = (45.0 + theta / 2.0).to_radians();
                (phi, DEGREES_PER_RADIAN * half.tan().ln())
            }
            Projection::CylindricalEqualArea => {
                let lambda = params.cea_lambda.unwrap_or(1.0);
                (phi, DEGREES_PER_RADIAN * theta.to_radians().sin() / lambda)
            }
            Projection::HammerAitoff => {
                let (phi, theta) = (phi.to_radians(), theta.to_radians());
                let denominator = 1.0 + theta.cos() * (phi / 2.0).cos();

                if denominator <= 0.0 {
                    return (f64::NAN, f64::NAN);
                }

                let gamma = (2.0 / denominator).sqrt();

                (
                    DEGREES_PER_RADIAN * 2.0 * gamma * theta.cos() * (phi / 2.0).sin(),
                    DEGREES_PER_RADIAN * gamma * theta.sin(),
                )
            }
            Projection::Mollweide => {
                let (phi, theta) = (phi.to_radians(), theta.to_radians());
                let gamma = mollweide_parametric(theta);
                let root_two = std::f64::consts::SQRT_2;

                (
                    DEGREES_PER_RADIAN * 2.0 * root_two * phi * gamma.cos() / std::f64::consts::PI,
                    DEGREES_PER_RADIAN * root_two * gamma.sin(),
                )
            }
            Projection::Linear
            | Projection::Gnomonic
            | Projection::Orthographic
            | Projection::Stereographic
            | Projection::ZenithalEquidistant
            | Projection::ZenithalEqualArea => (phi, theta),
        }
    }

    /// The native latitude a zenithal projection puts `radius` degrees from its
    /// pole.
    fn zenithal_theta(self, radius: f64) -> f64 {
        match self {
            // The tangent plane touches the sphere at the pole, so the radius
            // is the cotangent of the latitude.
            Projection::Gnomonic => DEGREES_PER_RADIAN.atan2(radius).to_degrees(),
            Projection::Orthographic => arccosine(radius / DEGREES_PER_RADIAN),
            Projection::Stereographic => {
                90.0 - 2.0 * (radius / (2.0 * DEGREES_PER_RADIAN)).atan().to_degrees()
            }
            Projection::ZenithalEquidistant => 90.0 - radius,
            Projection::ZenithalEqualArea => {
                90.0 - 2.0 * arcsine(radius / (2.0 * DEGREES_PER_RADIAN))
            }
            _ => f64::NAN,
        }
    }

    /// How far from its pole a zenithal projection puts native latitude
    /// `theta`, in degrees.
    fn zenithal_radius(self, theta: f64) -> f64 {
        let theta = theta.to_radians();

        match self {
            Projection::Gnomonic => {
                let tan = theta.tan();
                // The equator and the far hemisphere have no gnomonic image at
                // all: the ray through them never meets the tangent plane.
                if tan <= 0.0 {
                    f64::NAN
                } else {
                    DEGREES_PER_RADIAN / tan
                }
            }
            Projection::Orthographic => {
                if theta < 0.0 {
                    f64::NAN
                } else {
                    DEGREES_PER_RADIAN * theta.cos()
                }
            }
            Projection::Stereographic => {
                let half = (std::f64::consts::FRAC_PI_4 - theta / 2.0).tan();
                2.0 * DEGREES_PER_RADIAN * half
            }
            Projection::ZenithalEquidistant => 90.0 - theta.to_degrees(),
            Projection::ZenithalEqualArea => {
                2.0 * DEGREES_PER_RADIAN * (std::f64::consts::FRAC_PI_4 - theta / 2.0).sin()
            }
            _ => f64::NAN,
        }
    }
}

/// `asin` in degrees, giving `NaN` outside the domain rather than clamping — a
/// clamped value is a coordinate in the wrong place.
fn arcsine(value: f64) -> f64 {
    if !(-1.0..=1.0).contains(&value) {
        f64::NAN
    } else {
        value.asin().to_degrees()
    }
}

/// `acos` in degrees, with the same treatment of the domain.
fn arccosine(value: f64) -> f64 {
    if !(-1.0..=1.0).contains(&value) {
        f64::NAN
    } else {
        value.acos().to_degrees()
    }
}

/// Solves `2y + sin 2y = pi sin(theta)` for the parametric latitude Mollweide
/// is drawn in.
///
/// The equation has no closed form, so it is solved by Newton's method, which
/// converges in a handful of steps everywhere but the poles.
fn mollweide_parametric(theta: f64) -> f64 {
    let target = std::f64::consts::PI * theta.sin();

    // At the poles the equation is satisfied exactly by the pole itself, where
    // the derivative vanishes and Newton's method would not move.
    if (theta.abs() - std::f64::consts::FRAC_PI_2).abs() < 1e-12 {
        return std::f64::consts::FRAC_PI_2.copysign(theta);
    }

    let mut gamma = theta;
    for _ in 0..32 {
        let residual = 2.0 * gamma + (2.0 * gamma).sin() - target;
        let derivative = 2.0 + 2.0 * (2.0 * gamma).cos();

        if derivative.abs() < 1e-12 {
            break;
        }

        let step = residual / derivative;
        gamma -= step;

        if step.abs() < 1e-14 {
            break;
        }
    }

    gamma
}

#[cfg(test)]
mod tests {
    use super::{Projection, ProjectionParams};

    /// Every projection that carries a sphere, with the native latitude range
    /// it can represent.
    const SPHERICAL: [Projection; 10] = [
        Projection::Gnomonic,
        Projection::Orthographic,
        Projection::Stereographic,
        Projection::ZenithalEquidistant,
        Projection::ZenithalEqualArea,
        Projection::PlateCarree,
        Projection::Mercator,
        Projection::CylindricalEqualArea,
        Projection::HammerAitoff,
        Projection::Mollweide,
    ];

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
        assert_eq!(
            Projection::from_ctype("RA---SIN").unwrap(),
            Projection::Orthographic
        );
        assert_eq!(
            Projection::from_ctype("GLON-AIT").unwrap(),
            Projection::HammerAitoff
        );
    }

    #[test]
    fn a_distorted_gnomonic_axis_is_still_gnomonic() {
        // TPV is TAN with a polynomial correction, which the distortion applies
        // before the projection sees the coordinates.
        assert_eq!(
            Projection::from_ctype("RA---TPV").unwrap(),
            Projection::Gnomonic
        );
    }

    #[test]
    fn an_unimplemented_projection_is_an_error_rather_than_a_wrong_answer() {
        // Falling back to a linear mapping here would hand back coordinates that
        // look entirely reasonable and are in the wrong place.
        let error =
            Projection::from_ctype("RA---COE").expect_err("the COE projection is not implemented");

        assert!(error.to_string().contains("COE"), "got: {error}");
    }

    #[test]
    fn every_projection_round_trips_between_the_plane_and_its_native_sphere() {
        let params = ProjectionParams::default();

        for projection in SPHERICAL {
            for phi in [-150.0, -30.0, 0.0, 45.0, 170.0] {
                for theta in [-60.0, -10.0, 0.0, 25.0, 80.0] {
                    let (x, y) = projection.from_native(phi, theta, &params);

                    // A projection that cannot draw this point says so, and
                    // there is nothing to round trip.
                    if x.is_nan() || y.is_nan() {
                        continue;
                    }

                    let (back_phi, back_theta) = projection.to_native(x, y, &params);

                    assert!(
                        (back_phi - phi).abs() < 1e-8 && (back_theta - theta).abs() < 1e-8,
                        "{:?} took ({phi}, {theta}) to ({x}, {y}) and back to ({back_phi}, \
                         {back_theta})",
                        projection
                    );
                }
            }
        }
    }

    #[test]
    fn the_fiducial_point_is_at_the_origin_of_the_plane() {
        let params = ProjectionParams::default();

        for projection in SPHERICAL {
            let (phi, theta) = projection.fiducial();
            let (x, y) = projection.from_native(phi, theta, &params);

            assert!(
                x.abs() < 1e-9 && y.abs() < 1e-9,
                "{:?} puts its reference point at ({x}, {y})",
                projection
            );
        }
    }

    #[test]
    fn a_point_the_projection_cannot_draw_is_not_quietly_moved() {
        let params = ProjectionParams::default();

        // The far hemisphere has no gnomonic image: every ray through it runs
        // away from the tangent plane.
        let (x, y) = Projection::Gnomonic.from_native(0.0, -30.0, &params);
        assert!(x.is_nan() && y.is_nan(), "got ({x}, {y})");

        // Orthographic sees only the near hemisphere.
        let (x, y) = Projection::Orthographic.from_native(0.0, -1.0, &params);
        assert!(x.is_nan() && y.is_nan(), "got ({x}, {y})");

        // Outside the Hammer-Aitoff ellipse there is no sky at all.
        let (phi, theta) = Projection::HammerAitoff.to_native(180.0, 120.0, &params);
        assert!(phi.is_nan() && theta.is_nan(), "got ({phi}, {theta})");
    }

    #[test]
    fn the_plate_carree_is_longitude_and_latitude_unchanged() {
        let params = ProjectionParams::default();

        assert_eq!(
            Projection::PlateCarree.to_native(30.0, -20.0, &params),
            (30.0, -20.0)
        );
    }
}
