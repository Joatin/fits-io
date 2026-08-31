use crate::header::Header;
use crate::wcs::Projection;
use crate::wcs::distortion::{Distortion, number};
use crate::wcs::projection::ProjectionParams;
use crate::wcs::spherical::Rotation;
use std::error::Error;

/// The world coordinate system of an image.
///
/// Built from a header's CRPIXn, CRVALn, CDELTn, CDi_j, CROTAn and CTYPEn cards,
/// this converts between pixel positions and the sky coordinates they fall on.
/// The first two axes carry the projection, and [`Wcs::pixel_to_world`] works on
/// those; a cube's remaining axes are read one at a time through
/// [`Wcs::pixel_to_world_axis`].
///
/// Pixel coordinates follow the FITS convention: the centre of the first pixel
/// is `(1.0, 1.0)`, not `(0.0, 0.0)`. Use [`Wcs::pixel_to_world_indexed`] and
/// [`Wcs::world_to_pixel_indexed`] to work in zero-based array indices instead.
///
/// A pixel the projection cannot place on the sky — a corner of an all-sky
/// image falls outside the sky itself — comes back as `NaN` rather than as a
/// plausible coordinate somewhere else.
#[derive(Debug, Clone, PartialEq)]
pub struct Wcs {
    reference_pixel: (f64, f64),
    reference_value: (f64, f64),
    /// Row-major, taking pixel offsets to intermediate world coordinates.
    transform: [[f64; 2]; 2],
    /// Its inverse, worked out once so that `world_to_pixel` need not.
    inverse: [[f64; 2]; 2],
    projection: Projection,
    /// How the projection's own sphere sits against the celestial one. A linear
    /// system has no sphere and no rotation.
    rotation: Option<Rotation>,
    params: ProjectionParams,
    distortion: Distortion,
    /// Every axis the header describes, including the two the projection uses.
    axes: Vec<Axis>,
}

/// One axis of the coordinate system, as its own CRPIXn, CRVALn and CDELTn
/// describe it.
#[derive(Debug, Clone, PartialEq)]
struct Axis {
    reference_pixel: f64,
    reference_value: f64,
    delta: f64,
    ctype: Option<String>,
    cunit: Option<String>,
}

impl Wcs {
    /// Reads the world coordinate system out of `header`.
    ///
    /// # Errors
    ///
    /// Returns an error when the header carries no usable WCS — CRPIXn and
    /// CRVALn are both required — when it names a projection this crate does not
    /// implement, or when its celestial axes are given in units other than
    /// degrees.
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
        let ctype = header.coordinate_axis_name(0).map(str::to_string);
        let projection = match ctype.as_deref() {
            Some(ctype) => Projection::from_ctype(projection_code(ctype))?,
            None => Projection::Linear,
        };

        let rotation = if projection == Projection::Linear {
            None
        } else {
            for index in 0..2 {
                degrees(header, index)?;
            }

            Some(Rotation::new(
                reference_value,
                projection.fiducial(),
                number(header, "LONPOLE").or_else(|| number(header, "PV1_3")),
                number(header, "LATPOLE").or_else(|| number(header, "PV1_4")),
            )?)
        };

        let params = ProjectionParams {
            cea_lambda: number(header, "PV2_1"),
        };

        Ok(Self {
            reference_pixel,
            reference_value,
            transform,
            inverse,
            projection,
            rotation,
            params,
            distortion: Distortion::from_header(header, ctype.as_deref()),
            axes: axes_of(header),
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

    /// Whether the first two axes describe a position on the sky, as opposed to
    /// a plain linear pair.
    pub fn is_celestial(&self) -> bool {
        self.rotation.is_some()
    }

    /// The celestial coordinates of the projection's own pole, in degrees, for a
    /// system that has one.
    ///
    /// For the zenithal projections — TAN among them — this is the reference
    /// point itself. For the whole-sky projections it is a quarter turn away
    /// from it, which is what an all-sky map is drawn about.
    pub fn celestial_pole(&self) -> Option<(f64, f64)> {
        self.rotation.as_ref().map(Rotation::pole)
    }

    /// How many axes the header describes.
    pub fn axis_count(&self) -> usize {
        self.axes.len()
    }

    /// The CTYPEn of an axis, counting from zero.
    pub fn axis_type(&self, axis: usize) -> Option<&str> {
        self.axes.get(axis)?.ctype.as_deref()
    }

    /// The CUNITn of an axis, counting from zero — the unit its world
    /// coordinates are in.
    pub fn axis_unit(&self, axis: usize) -> Option<&str> {
        self.axes.get(axis)?.cunit.as_deref()
    }

    /// The world coordinate at a pixel along one axis, counting from zero.
    ///
    /// This is how a cube's third axis is read: the wavelength, frequency or
    /// time a plane was taken at. `pixel` is one-based, as FITS counts pixels.
    ///
    /// A `-LOG` axis is read as the standard defines it, with the coordinate
    /// growing geometrically rather than by a fixed step. The two celestial
    /// axes have no coordinate of their own — they only mean anything together —
    /// so they come back `None`; [`Wcs::pixel_to_world`] is what reads those.
    pub fn pixel_to_world_axis(&self, axis: usize, pixel: f64) -> Option<f64> {
        if self.is_celestial() && axis < 2 {
            return None;
        }

        let described = self.axes.get(axis)?;
        let offset = described.delta * (pixel - described.reference_pixel);

        Some(if self.is_logarithmic(axis) {
            described.reference_value * (offset / described.reference_value).exp()
        } else {
            described.reference_value + offset
        })
    }

    /// The pixel a world coordinate falls on along one axis, the inverse of
    /// [`Wcs::pixel_to_world_axis`].
    pub fn world_to_pixel_axis(&self, axis: usize, world: f64) -> Option<f64> {
        if self.is_celestial() && axis < 2 {
            return None;
        }

        let described = self.axes.get(axis)?;

        if described.delta == 0.0 {
            return None;
        }

        let offset = if self.is_logarithmic(axis) {
            described.reference_value * (world / described.reference_value).ln()
        } else {
            world - described.reference_value
        };

        Some(described.reference_pixel + offset / described.delta)
    }

    /// Whether an axis grows geometrically, as its CTYPEn `-LOG` code says.
    fn is_logarithmic(&self, axis: usize) -> bool {
        self.axes
            .get(axis)
            .and_then(|axis| axis.ctype.as_deref())
            .is_some_and(|ctype| ctype.trim().ends_with("-LOG"))
    }

    /// The sky coordinate at a pixel, in degrees.
    ///
    /// `pixel` is one-based, as FITS counts pixels.
    pub fn pixel_to_world(&self, pixel: (f64, f64)) -> (f64, f64) {
        let intermediate = self.pixel_to_intermediate(pixel);

        let Some(rotation) = &self.rotation else {
            return (
                self.reference_value.0 + intermediate.0,
                self.reference_value.1 + intermediate.1,
            );
        };

        let (phi, theta) = self
            .projection
            .to_native(intermediate.0, intermediate.1, &self.params);

        rotation.to_celestial(phi, theta)
    }

    /// The pixel a sky coordinate falls on, in degrees in and one-based pixels
    /// out.
    pub fn world_to_pixel(&self, world: (f64, f64)) -> (f64, f64) {
        let intermediate = match &self.rotation {
            None => (
                world.0 - self.reference_value.0,
                world.1 - self.reference_value.1,
            ),
            Some(rotation) => {
                let (phi, theta) = rotation.to_native(world.0, world.1);
                self.projection.from_native(phi, theta, &self.params)
            }
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

        // SIP corrects the pixel offsets, TPV the coordinates the matrix
        // produces, so each sits on its own side of it.
        let corrected = self.distortion.correct_pixel(offset);

        self.distortion
            .correct_intermediate(apply(self.transform, corrected))
    }

    /// The inverse of [`Wcs::pixel_to_intermediate`].
    fn intermediate_to_pixel(&self, intermediate: (f64, f64)) -> (f64, f64) {
        let undistorted = self.distortion.uncorrect_intermediate(intermediate);
        let offset = self
            .distortion
            .uncorrect_pixel(apply(self.inverse, undistorted));

        (
            self.reference_pixel.0 + offset.0,
            self.reference_pixel.1 + offset.1,
        )
    }
}

/// The part of a CTYPEn that names the projection, with any distortion code
/// taken off the end.
///
/// `RA---TAN-SIP` is the gnomonic projection with a polynomial correction, not a
/// projection called SIP.
fn projection_code(ctype: &str) -> &str {
    ctype.trim().strip_suffix("-SIP").unwrap_or(ctype.trim())
}

/// Checks that a celestial axis is given in degrees.
///
/// CDELTn and CRVALn mean nothing without their unit, and every formula here is
/// in degrees. A header measuring its axis in arcseconds and being read as
/// degrees is wrong by a factor of 3600.
fn degrees(header: &Header, index: usize) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(unit) = string(header, &format!("CUNIT{}", index + 1)) else {
        // No CUNITn at all means degrees, which is what the standard says.
        return Ok(());
    };

    match unit.trim() {
        "deg" | "degree" | "degrees" | "" => Ok(()),
        other => Err(format!(
            "The celestial axis CUNIT{} is {:?}, and this crate reads celestial coordinates in \
             degrees",
            index + 1,
            other
        )
        .into()),
    }
}

/// The text a card holds, for the keywords with no typed accessor of their own.
fn string(header: &Header, key: &str) -> Option<String> {
    match header.card(key)? {
        crate::header::Value::String { value, .. } => Some(value),
        _ => None,
    }
}

/// Reads every axis the header describes, in order.
fn axes_of(header: &Header) -> Vec<Axis> {
    let count = header.naxis().unwrap_or(0).max(0) as usize;

    (0..count)
        .map(|index| Axis {
            reference_pixel: header.coordinate_reference_pixel(index).unwrap_or(0.0),
            reference_value: header.coordinate_value_at_pixel(index).unwrap_or(0.0),
            // A missing CDELTn is one unit per pixel, as the standard says.
            delta: header.coordinate_delta(index).unwrap_or(1.0),
            ctype: header.coordinate_axis_name(index).map(str::to_string),
            cunit: string(header, &format!("CUNIT{}", index + 1)),
        })
        .collect()
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
        let identity = |row: usize, column: usize| if row == column { 1.0 } else { 0.0 };
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

#[cfg(test)]
mod tests {
    use super::{Wcs, invert};
    use crate::header::Header;
    use crate::wcs::spherical::normalise_longitude;

    /// A header describing a two-axis image under `projection`, rotated by
    /// `rotation` degrees, the way a CDELTn and CROTAn header would say it.
    fn header(projection: &str, rotation: f64) -> Header {
        let mut header = Header::default();

        header.set_card("NAXIS", 2_i64).unwrap();
        header.set_card("CRPIX1", 100.5).unwrap();
        header.set_card("CRPIX2", 200.5).unwrap();
        header.set_card("CRVAL1", 150.0).unwrap();
        header.set_card("CRVAL2", 40.0).unwrap();
        header.set_card("CDELT1", -0.001).unwrap();
        header.set_card("CDELT2", 0.001).unwrap();
        header.set_card("CROTA2", rotation).unwrap();
        header
            .set_card("CTYPE1", format!("RA---{projection}"))
            .unwrap();
        header
            .set_card("CTYPE2", format!("DEC--{projection}"))
            .unwrap();

        header
    }

    fn wcs(projection: &str, rotation: f64) -> Wcs {
        Wcs::from_header(&header(projection, rotation)).expect("a complete WCS header")
    }

    fn assert_close(actual: (f64, f64), expected: (f64, f64)) {
        assert!(
            (actual.0 - expected.0).abs() < 1e-9 && (actual.1 - expected.1).abs() < 1e-9,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn the_reference_pixel_sits_at_the_reference_value() {
        for projection in ["TAN", "SIN", "ARC", "STG", "ZEA", "CAR", "AIT", "MOL"] {
            let wcs = wcs(projection, 0.0);
            assert_close(wcs.pixel_to_world((100.5, 200.5)), (150.0, 40.0));
        }
    }

    #[test]
    fn pixel_and_world_round_trip() {
        for projection in [
            "TAN", "SIN", "ARC", "STG", "ZEA", "CAR", "MER", "AIT", "MOL",
        ] {
            for rotation in [0.0, 30.0, -12.5] {
                let wcs = wcs(projection, rotation);

                for pixel in [(1.0, 1.0), (100.5, 200.5), (512.0, 480.0)] {
                    let world = wcs.pixel_to_world(pixel);
                    assert!(
                        world.0.is_finite(),
                        "{projection} put pixel {pixel:?} nowhere"
                    );

                    // A millionth of a pixel: the projections that are solved
                    // by iteration rather than in closed form give up a few
                    // digits, and this is far below what any of it means.
                    let back = wcs.world_to_pixel(world);
                    assert!(
                        (back.0 - pixel.0).abs() < 1e-6 && (back.1 - pixel.1).abs() < 1e-6,
                        "{projection} took {pixel:?} to {world:?} and back to {back:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_linear_axis_is_a_plain_offset_from_the_reference() {
        let mut header = header("TAN", 0.0);
        header.set_card("CTYPE1", "LINEAR").unwrap();
        header.set_card("CTYPE2", "LINEAR").unwrap();

        let wcs = Wcs::from_header(&header).unwrap();

        // Ten pixels along the first axis, at -0.001 degrees per pixel.
        assert_close(wcs.pixel_to_world((110.5, 200.5)), (149.99, 40.0));
        assert!(!wcs.is_celestial());
    }

    #[test]
    fn a_gnomonic_axis_is_not_a_plain_offset() {
        // The whole point of a projection is that it is not linear: away from
        // the reference point, right ascension converges with latitude.
        let mut linear = header("TAN", 0.0);
        linear.set_card("CTYPE1", "LINEAR").unwrap();
        linear.set_card("CTYPE2", "LINEAR").unwrap();

        let linear = Wcs::from_header(&linear)
            .unwrap()
            .pixel_to_world((1100.5, 200.5));
        let gnomonic = wcs("TAN", 0.0).pixel_to_world((1100.5, 200.5));

        assert!(
            (linear.0 - gnomonic.0).abs() > 1e-6,
            "TAN and linear should disagree a degree from the reference, got {linear:?} and {gnomonic:?}"
        );
    }

    #[test]
    fn north_is_up_and_east_is_left_in_an_unrotated_image() {
        let wcs = wcs("TAN", 0.0);

        // A pixel above the reference is north of it, and one to the right is
        // west — which is what the negative CDELT1 of a sky image means.
        let north = wcs.pixel_to_world((100.5, 300.5));
        let right = wcs.pixel_to_world((200.5, 200.5));

        assert!(
            north.1 > 40.0,
            "north of the reference should be, got {north:?}"
        );
        assert!(
            (north.0 - 150.0).abs() < 1e-9,
            "straight north keeps its right ascension, got {north:?}"
        );
        assert!(
            right.0 < 150.0,
            "the right of the frame is west, got {right:?}"
        );
    }

    #[test]
    fn the_projections_agree_close_to_the_reference_point() {
        // Every projection is locally the same to first order, so a pixel a few
        // arcseconds out lands in the same place whichever one is used. A
        // projection with its formulae the wrong way round shows up here.
        let reference = wcs("TAN", 0.0).pixel_to_world((110.5, 210.5));

        for projection in ["SIN", "ARC", "STG", "ZEA"] {
            let other = wcs(projection, 0.0).pixel_to_world((110.5, 210.5));

            assert!(
                (other.0 - reference.0).abs() < 1e-6 && (other.1 - reference.1).abs() < 1e-6,
                "{projection} put the pixel at {other:?}, TAN at {reference:?}"
            );
        }
    }

    /// The closed-form gnomonic deprojection, written out independently of
    /// everything the crate does, so that the general machinery has something
    /// to be checked against.
    fn gnomonic_by_hand(reference: (f64, f64), xi: f64, eta: f64) -> (f64, f64) {
        let (xi, eta) = (xi.to_radians(), eta.to_radians());
        let (longitude, latitude) = (reference.0.to_radians(), reference.1.to_radians());
        let (sin, cos) = latitude.sin_cos();

        let denominator = cos - eta * sin;

        (
            normalise_longitude((longitude + xi.atan2(denominator)).to_degrees()),
            ((sin + eta * cos) / (xi * xi + denominator * denominator).sqrt())
                .atan()
                .to_degrees(),
        )
    }

    #[test]
    fn the_gnomonic_projection_agrees_with_its_closed_form() {
        let wcs = wcs("TAN", 0.0);

        for pixel in [(1.0, 1.0), (250.0, 60.0), (900.5, 1000.5)] {
            // The intermediate coordinates the matrix produces, by hand.
            let (u, v) = (pixel.0 - 100.5, pixel.1 - 200.5);
            let expected = gnomonic_by_hand((150.0, 40.0), -0.001 * u, 0.001 * v);

            let actual = wcs.pixel_to_world(pixel);

            assert!(
                (actual.0 - expected.0).abs() < 1e-10 && (actual.1 - expected.1).abs() < 1e-10,
                "at {pixel:?} the closed form gives {expected:?} and the projection {actual:?}"
            );
        }
    }

    #[test]
    fn indexed_helpers_shift_between_pixel_conventions() {
        let wcs = wcs("TAN", 0.0);

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
        let wcs = wcs("TAN", 0.0);

        let world = wcs.pixel_to_world_indexed((400, 400));
        assert_eq!(wcs.world_to_pixel_indexed(world, 100, 100), None);
    }

    #[test]
    fn a_cubes_third_axis_reads_on_its_own() {
        let mut header = header("TAN", 0.0);
        header.set_card("NAXIS", 3_i64).unwrap();
        header.set_card("CTYPE3", "WAVE").unwrap();
        header.set_card("CUNIT3", "Angstrom").unwrap();
        header.set_card("CRPIX3", 1.0).unwrap();
        header.set_card("CRVAL3", 4000.0).unwrap();
        header.set_card("CDELT3", 1.25).unwrap();

        let wcs = Wcs::from_header(&header).unwrap();

        assert_eq!(wcs.axis_count(), 3);
        assert_eq!(wcs.axis_type(2), Some("WAVE"));
        assert_eq!(wcs.axis_unit(2), Some("Angstrom"));
        assert_eq!(wcs.pixel_to_world_axis(2, 1.0), Some(4000.0));
        assert_eq!(wcs.pixel_to_world_axis(2, 5.0), Some(4005.0));
        assert_eq!(wcs.world_to_pixel_axis(2, 4005.0), Some(5.0));

        // The celestial axes only mean anything together.
        assert_eq!(wcs.pixel_to_world_axis(0, 1.0), None);
        assert_eq!(wcs.pixel_to_world_axis(9, 1.0), None);
    }

    #[test]
    fn a_logarithmic_axis_grows_geometrically() {
        let mut header = header("TAN", 0.0);
        header.set_card("NAXIS", 3_i64).unwrap();
        header.set_card("CTYPE3", "FREQ-LOG").unwrap();
        header.set_card("CRPIX3", 1.0).unwrap();
        header.set_card("CRVAL3", 1000.0).unwrap();
        header.set_card("CDELT3", 10.0).unwrap();

        let wcs = Wcs::from_header(&header).unwrap();

        let at_reference = wcs.pixel_to_world_axis(2, 1.0).unwrap();
        let further = wcs.pixel_to_world_axis(2, 11.0).unwrap();

        assert!((at_reference - 1000.0).abs() < 1e-9);
        assert!((further - 1000.0 * (100.0_f64 / 1000.0).exp()).abs() < 1e-6);
        assert!((wcs.world_to_pixel_axis(2, further).unwrap() - 11.0).abs() < 1e-9);
    }

    #[test]
    fn a_celestial_axis_in_the_wrong_unit_is_refused() {
        let mut header = header("TAN", 0.0);
        header.set_card("CUNIT1", "arcsec").unwrap();

        let error = Wcs::from_header(&header).expect_err("arcseconds are not degrees");
        assert!(error.to_string().contains("arcsec"), "got: {error}");
    }

    #[test]
    fn a_sip_header_bends_the_field_and_bends_it_back() {
        let mut header = header("TAN", 0.0);
        header.set_card("CTYPE1", "RA---TAN-SIP").unwrap();
        header.set_card("CTYPE2", "DEC--TAN-SIP").unwrap();
        header.set_card("A_ORDER", 2_i64).unwrap();
        header.set_card("A_2_0", 1e-5).unwrap();
        header.set_card("B_ORDER", 2_i64).unwrap();
        header.set_card("B_0_2", -2e-5).unwrap();

        let distorted = Wcs::from_header(&header).unwrap();
        let ideal = wcs("TAN", 0.0);

        let pixel = (600.5, 700.5);

        let with = distorted.pixel_to_world(pixel);
        let without = ideal.pixel_to_world(pixel);

        assert!(
            (with.0 - without.0).abs() > 1e-6 || (with.1 - without.1).abs() > 1e-6,
            "the correction should move the corner of the frame, got {with:?} and {without:?}"
        );

        let back = distorted.world_to_pixel(with);
        assert!(
            (back.0 - pixel.0).abs() < 1e-6 && (back.1 - pixel.1).abs() < 1e-6,
            "{pixel:?} came back as {back:?}"
        );
    }

    #[test]
    fn a_tpv_header_bends_the_field_and_bends_it_back() {
        let mut header = header("TAN", 0.0);
        header.set_card("CTYPE1", "RA---TPV").unwrap();
        header.set_card("CTYPE2", "DEC--TPV").unwrap();
        header.set_card("PV1_1", 1.0).unwrap();
        header.set_card("PV1_4", 0.002).unwrap();
        header.set_card("PV2_1", 1.0).unwrap();

        let wcs = Wcs::from_header(&header).unwrap();
        let pixel = (600.5, 700.5);

        let world = wcs.pixel_to_world(pixel);
        let back = wcs.world_to_pixel(world);

        assert!(
            (back.0 - pixel.0).abs() < 1e-6 && (back.1 - pixel.1).abs() < 1e-6,
            "{pixel:?} came back as {back:?}"
        );
    }

    #[test]
    fn an_all_sky_projection_puts_its_pole_a_quarter_turn_from_its_centre() {
        let mut header = header("AIT", 0.0);
        header.set_card("CRVAL1", 0.0).unwrap();
        header.set_card("CRVAL2", 0.0).unwrap();

        let wcs = Wcs::from_header(&header).unwrap();

        let pole = wcs.celestial_pole().expect("a celestial system has a pole");
        assert!((pole.1 - 90.0).abs() < 1e-9, "got {pole:?}");
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
