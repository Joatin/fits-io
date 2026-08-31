//! The rotation between a projection's native sphere and the celestial one.
//!
//! A projection draws its own sphere, whose pole is wherever the projection is
//! centred. Turning that into right ascension and declination is a rotation,
//! fixed by three things the header gives: the reference point CRVALn, which
//! native position it corresponds to, and LONPOLE — the native longitude of the
//! celestial pole, which says how the frame is turned about the reference point.

use std::error::Error;

/// The rotation carrying a projection's native sphere onto the celestial one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rotation {
    /// The celestial coordinates of the native pole, in degrees.
    pole: (f64, f64),
    /// The native longitude of the celestial pole, in degrees.
    native_pole_longitude: f64,
}

impl Rotation {
    /// The rotation a header describes.
    ///
    /// `reference` is CRVALn, `fiducial` the native coordinates the projection
    /// puts the reference point at, and `lonpole` and `latpole` the cards of
    /// those names where the header carries them.
    ///
    /// # Errors
    ///
    /// Returns an error when the header's LONPOLE cannot be reconciled with its
    /// reference point — which describes no orientation of the sphere at all,
    /// rather than an unusual one.
    pub(crate) fn new(
        reference: (f64, f64),
        fiducial: (f64, f64),
        lonpole: Option<f64>,
        latpole: Option<f64>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let (alpha_0, delta_0) = (reference.0.to_radians(), reference.1.to_radians());
        let (phi_0, theta_0) = (fiducial.0.to_radians(), fiducial.1.to_radians());

        // The standard's default: the celestial pole is behind the reference
        // point when the reference is south of the projection's own latitude,
        // and in front of it otherwise.
        let phi_p = lonpole
            .unwrap_or(if reference.1 >= fiducial.1 {
                0.0
            } else {
                180.0
            })
            .to_radians();

        // A zenithal projection is centred on its reference point, so the native
        // pole *is* the reference point and there is nothing to solve.
        if (theta_0 - std::f64::consts::FRAC_PI_2).abs() < 1e-12 {
            return Ok(Self {
                pole: reference,
                native_pole_longitude: phi_p.to_degrees(),
            });
        }

        let (sin_theta_0, cos_theta_0) = theta_0.sin_cos();
        let cos_delta_phi = (phi_p - phi_0).cos();

        let scale = (sin_theta_0 * sin_theta_0
            + cos_theta_0 * cos_theta_0 * cos_delta_phi * cos_delta_phi)
            .sqrt();

        if scale == 0.0 {
            return Err(format!(
                "A LONPOLE of {} leaves the reference point free to turn about the sky, so the \
                 header describes no one orientation",
                phi_p.to_degrees()
            )
            .into());
        }

        let ratio = delta_0.sin() / scale;
        if !(-1.0..=1.0).contains(&ratio) {
            return Err(format!(
                "No celestial pole can sit at LONPOLE {} while the reference point is at \
                 declination {}",
                phi_p.to_degrees(),
                reference.1
            )
            .into());
        }

        let centre = sin_theta_0.atan2(cos_theta_0 * cos_delta_phi);
        let spread = ratio.acos();

        // Two poles satisfy the header; the one nearer LATPOLE is the one it
        // means, and LATPOLE defaults to the north pole.
        let candidates = [centre + spread, centre - spread];
        let wanted = latpole.unwrap_or(90.0).to_radians();

        let delta_p = candidates
            .into_iter()
            .filter(|angle| angle.abs() <= std::f64::consts::FRAC_PI_2 + 1e-12)
            .min_by(|a, b| {
                (a - wanted)
                    .abs()
                    .partial_cmp(&(b - wanted).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| {
                format!(
                    "The reference point at declination {} cannot be placed under this \
                     projection's LONPOLE",
                    reference.1
                )
            })?;

        let alpha_p = alpha_0
            - (((phi_p - phi_0).sin() * cos_theta_0)
                .atan2(sin_theta_0 * delta_p.cos() - cos_theta_0 * delta_p.sin() * cos_delta_phi));

        Ok(Self {
            pole: (
                normalise_longitude(alpha_p.to_degrees()),
                delta_p.to_degrees(),
            ),
            native_pole_longitude: phi_p.to_degrees(),
        })
    }

    /// The celestial coordinates of a native position, both in degrees.
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn to_celestial(&self, phi: f64, theta: f64) -> (f64, f64) {
        if phi.is_nan() || theta.is_nan() {
            return (f64::NAN, f64::NAN);
        }

        let (phi, theta) = (phi.to_radians(), theta.to_radians());
        let (alpha_p, delta_p) = (self.pole.0.to_radians(), self.pole.1.to_radians());
        let delta_phi = phi - self.native_pole_longitude.to_radians();

        let (sin_theta, cos_theta) = theta.sin_cos();
        let (sin_pole, cos_pole) = delta_p.sin_cos();
        let (sin_delta_phi, cos_delta_phi) = delta_phi.sin_cos();

        // The two components of the celestial position perpendicular to the
        // pole, which give both the right ascension and — through their
        // length — a declination that stays accurate right up to the pole.
        // `asin` of the third component alone loses half its digits there,
        // which for a narrow field is the whole answer.
        let east = -cos_theta * sin_delta_phi;
        let north = sin_theta * cos_pole - cos_theta * sin_pole * cos_delta_phi;
        let along = sin_theta * sin_pole + cos_theta * cos_pole * cos_delta_phi;

        let alpha = alpha_p + east.atan2(north);
        let delta = along.atan2(east.hypot(north));

        (normalise_longitude(alpha.to_degrees()), delta.to_degrees())
    }

    /// The native position of a celestial coordinate, the inverse of
    /// [`Rotation::to_celestial`].
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn to_native(&self, alpha: f64, delta: f64) -> (f64, f64) {
        let (alpha, delta) = (alpha.to_radians(), delta.to_radians());
        let (alpha_p, delta_p) = (self.pole.0.to_radians(), self.pole.1.to_radians());
        let delta_alpha = alpha - alpha_p;

        let (sin_delta, cos_delta) = delta.sin_cos();
        let (sin_pole, cos_pole) = delta_p.sin_cos();
        let (sin_delta_alpha, cos_delta_alpha) = delta_alpha.sin_cos();

        // As in `to_celestial`, the native latitude comes from the ratio of
        // the components rather than from the sine alone: a narrow field sits
        // within an arcminute of the native pole, where `asin` is at its worst.
        let east = -cos_delta * sin_delta_alpha;
        let north = sin_delta * cos_pole - cos_delta * sin_pole * cos_delta_alpha;
        let along = sin_delta * sin_pole + cos_delta * cos_pole * cos_delta_alpha;

        let phi = self.native_pole_longitude.to_radians() + east.atan2(north);
        let theta = along.atan2(east.hypot(north));

        // Native longitude runs from -180 to 180, which is the range every
        // projection here draws.
        (wrap_signed(phi.to_degrees()), theta.to_degrees())
    }

    /// The celestial coordinates of the native pole.
    pub(crate) fn pole(&self) -> (f64, f64) {
        self.pole
    }
}

/// Wraps a longitude into `0.0..360.0`.
pub(crate) fn normalise_longitude(degrees: f64) -> f64 {
    if !degrees.is_finite() {
        return degrees;
    }

    let wrapped = degrees % 360.0;
    if wrapped < 0.0 {
        wrapped + 360.0
    } else {
        wrapped
    }
}

/// Wraps an angle into `-180.0..=180.0`.
fn wrap_signed(degrees: f64) -> f64 {
    let wrapped = normalise_longitude(degrees);
    if wrapped > 180.0 {
        wrapped - 360.0
    } else {
        wrapped
    }
}

#[cfg(test)]
mod tests {
    use super::Rotation;

    fn assert_close(actual: (f64, f64), expected: (f64, f64)) {
        assert!(
            (actual.0 - expected.0).abs() < 1e-9 && (actual.1 - expected.1).abs() < 1e-9,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn a_zenithal_projection_is_centred_on_its_reference_point() {
        let rotation = Rotation::new((150.0, 40.0), (0.0, 90.0), None, None).unwrap();

        assert_close(rotation.pole(), (150.0, 40.0));
        // The native pole is the reference point.
        assert_close(rotation.to_celestial(0.0, 90.0), (150.0, 40.0));
    }

    #[test]
    fn a_cylindrical_projection_puts_its_origin_at_the_reference_point() {
        for reference in [(150.0, 40.0), (10.0, -25.0), (0.0, 0.0)] {
            let rotation = Rotation::new(reference, (0.0, 0.0), None, None).unwrap();

            assert_close(rotation.to_celestial(0.0, 0.0), reference);
        }
    }

    #[test]
    fn the_rotation_and_its_inverse_undo_each_other() {
        for fiducial in [(0.0, 90.0), (0.0, 0.0)] {
            for reference in [(150.0, 40.0), (10.0, -25.0), (300.0, 5.0)] {
                let rotation = Rotation::new(reference, fiducial, None, None).unwrap();

                for phi in [-170.0, -40.0, 0.0, 60.0, 179.0] {
                    for theta in [-80.0, -15.0, 0.0, 33.0, 88.0] {
                        let (alpha, delta) = rotation.to_celestial(phi, theta);
                        let back = rotation.to_native(alpha, delta);

                        assert!(
                            (back.0 - phi).abs() < 1e-8 && (back.1 - theta).abs() < 1e-8,
                            "({phi}, {theta}) came back as {back:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn north_stays_up_under_a_zenithal_projection() {
        // Native longitude 180 is the direction of the celestial pole for a
        // zenithal projection, so a point a degree along it is a degree north.
        let rotation = Rotation::new((150.0, 40.0), (0.0, 90.0), None, None).unwrap();

        assert_close(rotation.to_celestial(180.0, 89.0), (150.0, 41.0));
    }
}
