use crate::header::Header;
use crate::wcs::Projection;
use std::error::Error;

/// The world coordinate system of a two-axis image.
///
/// Built from a header's CRPIXn, CRVALn, CDELTn, CROTA2 and CTYPEn cards, this
/// converts between pixel positions and the sky coordinates they fall on.
///
/// Pixel coordinates follow the FITS convention: the centre of the first pixel
/// is `(1.0, 1.0)`, not `(0.0, 0.0)`. Use [`Wcs::pixel_to_world_indexed`] and
/// [`Wcs::world_to_pixel_indexed`] to work in zero-based array indices instead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wcs {
    reference_pixel: (f64, f64),
    reference_value: (f64, f64),
    /// Row-major, taking pixel offsets to intermediate world coordinates.
    transform: [[f64; 2]; 2],
    /// Its inverse, worked out once so that `world_to_pixel` need not.
    inverse: [[f64; 2]; 2],
    projection: Projection,
}

impl Wcs {
    /// Reads the WCS of the first two axes out of `header`.
    ///
    /// # Errors
    ///
    /// Returns an error when the header carries no usable WCS — CRPIXn and
    /// CRVALn are both required — or when it names a projection this crate does
    /// not implement.
    pub fn from_header(header: &Header) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let axis = |name: &str, value: Option<f64>, index: usize| {
            value.ok_or_else(|| {
                format!(
                    "Header has no {}{} card, so it carries no world coordinate system",
                    name,
                    index + 1
                )
            })
        };

        let reference_pixel = (
            axis("CRPIX", header.coordinate_reference_pixel(0), 0)?,
            axis("CRPIX", header.coordinate_reference_pixel(1), 1)?,
        );
        let reference_value = (
            axis("CRVAL", header.coordinate_value_at_pixel(0), 0)?,
            axis("CRVAL", header.coordinate_value_at_pixel(1), 1)?,
        );

        let transform = transform_from(header);

        // A matrix that cannot be inverted maps every pixel onto the same point,
        // so there is no coordinate system here to speak of.
        let inverse = invert(transform).ok_or_else(|| {
            format!(
                "The header's coordinate transformation matrix {:?} cannot be inverted, so it \
                 describes no usable world coordinate system",
                transform
            )
        })?;

        // Both axes must agree on the projection; the first one names it.
        let projection = match header.coordinate_axis_name(0) {
            Some(ctype) => Projection::from_ctype(ctype)?,
            None => Projection::Linear,
        };

        Ok(Self {
            reference_pixel,
            reference_value,
            transform,
            inverse,
            projection,
        })
    }

    /// The matrix taking pixel offsets to intermediate world coordinates,
    /// row-major.
    ///
    /// Whichever of the three conventions the header used — a CDi_j matrix, a
    /// PCi_j matrix with CDELTn, or CDELTn with CROTAn — this is what it came
    /// to.
    pub fn transform(&self) -> [[f64; 2]; 2] {
        self.transform
    }

    /// The projection this system uses.
    pub fn projection(&self) -> Projection {
        self.projection
    }

    /// The sky coordinate at a pixel, in degrees.
    ///
    /// `pixel` is one-based, as FITS counts pixels.
    pub fn pixel_to_world(&self, pixel: (f64, f64)) -> (f64, f64) {
        let intermediate = self.pixel_to_intermediate(pixel);

        match self.projection {
            Projection::Linear => (
                self.reference_value.0 + intermediate.0,
                self.reference_value.1 + intermediate.1,
            ),
            Projection::Gnomonic => self.gnomonic_to_world(intermediate),
        }
    }

    /// The pixel a sky coordinate falls on, in degrees in and one-based pixels
    /// out.
    pub fn world_to_pixel(&self, world: (f64, f64)) -> (f64, f64) {
        let intermediate = match self.projection {
            Projection::Linear => (
                world.0 - self.reference_value.0,
                world.1 - self.reference_value.1,
            ),
            Projection::Gnomonic => self.world_to_gnomonic(world),
        };

        self.intermediate_to_pixel(intermediate)
    }

    /// As [`Wcs::pixel_to_world`], but taking a zero-based array index.
    pub fn pixel_to_world_indexed(&self, index: (u32, u32)) -> (f64, f64) {
        self.pixel_to_world((index.0 as f64 + 1.0, index.1 as f64 + 1.0))
    }

    /// As [`Wcs::world_to_pixel`], but returning a zero-based array index.
    ///
    /// The index is rounded to the nearest pixel, and `None` when the
    /// coordinate falls outside the `width` by `height` image.
    pub fn world_to_pixel_indexed(
        &self,
        world: (f64, f64),
        width: u32,
        height: u32,
    ) -> Option<(u32, u32)> {
        let (x, y) = self.world_to_pixel(world);
        let x = (x - 1.0).round();
        let y = (y - 1.0).round();

        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
            return None;
        }

        let (x, y) = (x as u32, y as u32);
        (x < width && y < height).then_some((x, y))
    }

    /// Pixel offsets from the reference pixel, put through the transformation
    /// matrix into the intermediate world coordinates the projection works in.
    fn pixel_to_intermediate(&self, pixel: (f64, f64)) -> (f64, f64) {
        let offset = (
            pixel.0 - self.reference_pixel.0,
            pixel.1 - self.reference_pixel.1,
        );

        apply(self.transform, offset)
    }

    /// The inverse of [`Wcs::pixel_to_intermediate`].
    fn intermediate_to_pixel(&self, intermediate: (f64, f64)) -> (f64, f64) {
        let offset = apply(self.inverse, intermediate);

        (
            self.reference_pixel.0 + offset.0,
            self.reference_pixel.1 + offset.1,
        )
    }

    /// The gnomonic (TAN) deprojection, from plane offsets in degrees to a sky
    /// coordinate in degrees.
    fn gnomonic_to_world(&self, intermediate: (f64, f64)) -> (f64, f64) {
        let xi = intermediate.0.to_radians();
        let eta = intermediate.1.to_radians();

        let (reference_longitude, reference_latitude) = (
            self.reference_value.0.to_radians(),
            self.reference_value.1.to_radians(),
        );
        let (sin_latitude, cos_latitude) = reference_latitude.sin_cos();

        let denominator = cos_latitude - eta * sin_latitude;

        let longitude = reference_longitude + xi.atan2(denominator);
        let latitude = ((sin_latitude + eta * cos_latitude)
            / (xi * xi + denominator * denominator).sqrt())
        .atan();

        (
            normalise_longitude(longitude.to_degrees()),
            latitude.to_degrees(),
        )
    }

    /// The gnomonic (TAN) projection, the inverse of [`Wcs::gnomonic_to_world`].
    fn world_to_gnomonic(&self, world: (f64, f64)) -> (f64, f64) {
        let (longitude, latitude) = (world.0.to_radians(), world.1.to_radians());
        let (reference_longitude, reference_latitude) = (
            self.reference_value.0.to_radians(),
            self.reference_value.1.to_radians(),
        );

        let delta = longitude - reference_longitude;
        let (sin_latitude, cos_latitude) = latitude.sin_cos();
        let (sin_reference, cos_reference) = reference_latitude.sin_cos();

        // The cosine of the angle between the point and the reference. A point
        // on the far hemisphere has no gnomonic image at all, which shows up
        // here as a zero or negative denominator.
        let denominator = sin_latitude * sin_reference + cos_latitude * cos_reference * delta.cos();

        let xi = cos_latitude * delta.sin() / denominator;
        let eta = (sin_latitude * cos_reference - cos_latitude * sin_reference * delta.cos())
            / denominator;

        (xi.to_degrees(), eta.to_degrees())
    }
}

/// Reads the transformation matrix out of whichever convention the header uses.
///
/// FITS has three ways of saying the same thing, and they are tried in the order
/// the standard gives them:
///
/// 1. `CDi_j`, which carries the scale and the rotation together. This is what
///    most modern pipelines write, and when it is present CDELTn and CROTAn are
///    ignored.
/// 2. `PCi_j` scaled by `CDELTn`, which separates the rotation from the scale.
/// 3. `CDELTn` with `CROTAn`, the older convention, where the rotation is a
///    single angle.
///
/// Reading only the third and defaulting the scale to 1 leaves a `CDi_j` header
/// pointing a whole degree per pixel away from where it means.
fn transform_from(header: &Header) -> [[f64; 2]; 2] {
    let element = |row, column| header.coordinate_transform(row, column);

    // Any CDi_j at all means the header uses the CD convention; the elements it
    // leaves out are zero, as the standard says.
    if (0..2).any(|row| (0..2).any(|column| element(row, column).is_some())) {
        return [
            [element(0, 0).unwrap_or(0.0), element(0, 1).unwrap_or(0.0)],
            [element(1, 0).unwrap_or(0.0), element(1, 1).unwrap_or(0.0)],
        ];
    }

    // CDELTn defaults to 1, which is what the standard says a header with no
    // scale at all means.
    let scale = (
        header.coordinate_delta(0).unwrap_or(1.0),
        header.coordinate_delta(1).unwrap_or(1.0),
    );

    let rotation = |row, column| header.coordinate_rotation_matrix(row, column);

    if (0..2).any(|row| (0..2).any(|column| rotation(row, column).is_some())) {
        // A PCi_j the header leaves out is the identity matrix's value there.
        let identity = |row: usize, column: usize| {
            if row == column { 1.0 } else { 0.0 }
        };
        let element = |row, column| rotation(row, column).unwrap_or_else(|| identity(row, column));

        return [
            [scale.0 * element(0, 0), scale.0 * element(0, 1)],
            [scale.1 * element(1, 0), scale.1 * element(1, 1)],
        ];
    }

    // CROTAn is carried on the second axis by convention, but accept it on the
    // first for the headers that put it there.
    let angle = header
        .coordinate_rotation(1)
        .or_else(|| header.coordinate_rotation(0))
        .unwrap_or(0.0);
    let (sin, cos) = angle.to_radians().sin_cos();

    // The standard's relation between CROTAn and the matrix. Note which CDELT
    // goes with which element: the off-diagonal terms take the scale of the axis
    // they draw from, not the one they feed.
    [
        [scale.0 * cos, -scale.1 * sin],
        [scale.0 * sin, scale.1 * cos],
    ]
}

/// Multiplies a two-element offset by a matrix.
fn apply(matrix: [[f64; 2]; 2], offset: (f64, f64)) -> (f64, f64) {
    (
        matrix[0][0] * offset.0 + matrix[0][1] * offset.1,
        matrix[1][0] * offset.0 + matrix[1][1] * offset.1,
    )
}

/// Inverts a two-by-two matrix, or `None` if it is singular.
fn invert(matrix: [[f64; 2]; 2]) -> Option<[[f64; 2]; 2]> {
    let determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];

    if determinant == 0.0 || !determinant.is_finite() {
        return None;
    }

    Some([
        [matrix[1][1] / determinant, -matrix[0][1] / determinant],
        [-matrix[1][0] / determinant, matrix[0][0] / determinant],
    ])
}

/// Wraps a longitude into `0.0..360.0`.
fn normalise_longitude(degrees: f64) -> f64 {
    let wrapped = degrees % 360.0;
    if wrapped < 0.0 {
        wrapped + 360.0
    } else {
        wrapped
    }
}

#[cfg(test)]
mod tests {
    use super::{Wcs, invert, normalise_longitude};
    use crate::wcs::Projection;

    /// A system with the given projection and rotation, built the way a CROTAn
    /// header would express it.
    fn wcs(projection: Projection, rotation: f64) -> Wcs {
        let scale = (-0.001, 0.001);
        let (sin, cos) = f64::to_radians(rotation).sin_cos();

        let transform = [
            [scale.0 * cos, -scale.1 * sin],
            [scale.0 * sin, scale.1 * cos],
        ];

        Wcs {
            reference_pixel: (100.5, 200.5),
            reference_value: (150.0, 40.0),
            transform,
            inverse: super::invert(transform).expect("a rotation is invertible"),
            projection,
        }
    }

    fn assert_close(actual: (f64, f64), expected: (f64, f64)) {
        assert!(
            (actual.0 - expected.0).abs() < 1e-9 && (actual.1 - expected.1).abs() < 1e-9,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn the_reference_pixel_sits_at_the_reference_value() {
        for projection in [Projection::Linear, Projection::Gnomonic] {
            let wcs = wcs(projection, 0.0);
            assert_close(wcs.pixel_to_world((100.5, 200.5)), (150.0, 40.0));
        }
    }

    #[test]
    fn pixel_and_world_round_trip() {
        for projection in [Projection::Linear, Projection::Gnomonic] {
            for rotation in [0.0, 30.0, -12.5] {
                let wcs = wcs(projection, rotation);

                for pixel in [(1.0, 1.0), (100.5, 200.5), (512.0, 480.0)] {
                    let world = wcs.pixel_to_world(pixel);
                    assert_close(wcs.world_to_pixel(world), pixel);
                }
            }
        }
    }

    #[test]
    fn a_linear_axis_is_a_plain_offset_from_the_reference() {
        let wcs = wcs(Projection::Linear, 0.0);

        // Ten pixels along the first axis, at -0.001 degrees per pixel.
        assert_close(wcs.pixel_to_world((110.5, 200.5)), (149.99, 40.0));
    }

    #[test]
    fn a_gnomonic_axis_is_not_a_plain_offset() {
        // The whole point of a projection is that it is not linear: away from
        // the reference point, right ascension converges with latitude.
        let linear = wcs(Projection::Linear, 0.0).pixel_to_world((1100.5, 200.5));
        let gnomonic = wcs(Projection::Gnomonic, 0.0).pixel_to_world((1100.5, 200.5));

        assert!(
            (linear.0 - gnomonic.0).abs() > 1e-6,
            "TAN and linear should disagree a degree from the reference, got {linear:?} and {gnomonic:?}"
        );
    }

    #[test]
    fn indexed_helpers_shift_between_pixel_conventions() {
        let wcs = wcs(Projection::Gnomonic, 0.0);

        // Array index (0, 0) is FITS pixel (1.0, 1.0).
        assert_close(
            wcs.pixel_to_world_indexed((0, 0)),
            wcs.pixel_to_world((1.0, 1.0)),
        );

        let world = wcs.pixel_to_world_indexed((10, 20));
        assert_eq!(wcs.world_to_pixel_indexed(world, 512, 512), Some((10, 20)));
    }

    #[test]
    fn a_coordinate_outside_the_image_has_no_pixel() {
        let wcs = wcs(Projection::Gnomonic, 0.0);

        let world = wcs.pixel_to_world_indexed((400, 400));
        assert_eq!(wcs.world_to_pixel_indexed(world, 100, 100), None);
    }

    #[test]
    fn a_singular_matrix_has_no_inverse() {
        // Two axes that map onto the same line describe no coordinate system:
        // every pixel would land on the same place, and nothing maps back.
        assert!(invert([[1.0, 2.0], [2.0, 4.0]]).is_none());
        assert!(invert([[0.0, 0.0], [0.0, 0.0]]).is_none());

        assert!(invert([[1.0, 0.0], [0.0, 1.0]]).is_some());
    }

    #[test]
    fn longitudes_wrap_into_a_single_turn() {
        assert_eq!(normalise_longitude(370.0), 10.0);
        assert_eq!(normalise_longitude(-10.0), 350.0);
        assert_eq!(normalise_longitude(180.0), 180.0);
    }
}
