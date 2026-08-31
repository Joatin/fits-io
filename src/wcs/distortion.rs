//! The polynomial corrections a plate solver fits on top of a projection.
//!
//! A real telescope is not the ideal one a projection describes: the optics bend
//! the field, and the detector is not perfectly flat or square. Two conventions
//! for writing that down are common enough to meet in the wild, and both are
//! polynomials fitted alongside the WCS:
//!
//! * **SIP**, written by `astrometry.net` and most amateur plate solvers, which
//!   corrects the *pixel* offsets before the header's matrix sees them. A header
//!   using it says so in CTYPEn, as `RA---TAN-SIP`.
//! * **TPV**, written by `SCAMP` and the IRAF-descended pipelines, which
//!   corrects the *intermediate world coordinates* after the matrix. Its CTYPEn
//!   reads `RA---TPV`.
//!
//! Ignoring either leaves coordinates that are right at the centre of the frame
//! and wrong by seconds of arc at its corners.

use crate::header::{Header, Value};

/// A distortion sitting between the pixel grid and the projection.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Distortion {
    /// The header fits the ideal projection with no correction.
    None,
    /// Simple Imaging Polynomial, applied to pixel offsets from the reference
    /// pixel.
    Sip(Sip),
    /// The TPV polynomial, applied to intermediate world coordinates.
    Tpv(Box<Tpv>),
}

impl Distortion {
    /// Reads whichever distortion `header` carries, given the CTYPE1 it was read
    /// with.
    pub(crate) fn from_header(header: &Header, ctype: Option<&str>) -> Self {
        let ctype = ctype.unwrap_or_default().trim();

        if ctype.ends_with("-SIP")
            && let Some(sip) = Sip::from_header(header)
        {
            return Distortion::Sip(sip);
        }

        if ctype.ends_with("TPV")
            && let Some(tpv) = Tpv::from_header(header)
        {
            return Distortion::Tpv(Box::new(tpv));
        }

        Distortion::None
    }

    /// Corrects pixel offsets from the reference pixel, before the header's
    /// matrix is applied.
    pub(crate) fn correct_pixel(&self, offset: (f64, f64)) -> (f64, f64) {
        match self {
            Distortion::Sip(sip) => sip.correct(offset),
            _ => offset,
        }
    }

    /// Undoes [`Distortion::correct_pixel`].
    pub(crate) fn uncorrect_pixel(&self, corrected: (f64, f64)) -> (f64, f64) {
        match self {
            Distortion::Sip(sip) => sip.uncorrect(corrected),
            _ => corrected,
        }
    }

    /// Corrects intermediate world coordinates, after the header's matrix.
    pub(crate) fn correct_intermediate(&self, intermediate: (f64, f64)) -> (f64, f64) {
        match self {
            Distortion::Tpv(tpv) => tpv.correct(intermediate),
            _ => intermediate,
        }
    }

    /// Undoes [`Distortion::correct_intermediate`].
    pub(crate) fn uncorrect_intermediate(&self, corrected: (f64, f64)) -> (f64, f64) {
        match self {
            Distortion::Tpv(tpv) => invert_numerically(|point| tpv.correct(point), corrected),
            _ => corrected,
        }
    }
}

/// The SIP polynomials: `A` and `B` forwards, `AP` and `BP` back.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Sip {
    forward: (Polynomial, Polynomial),
    /// The fitted inverse, where the header carries one. A header without it is
    /// inverted by iterating the forward polynomial instead.
    inverse: Option<(Polynomial, Polynomial)>,
}

impl Sip {
    /// Reads the SIP coefficients, or `None` where the header has none.
    fn from_header(header: &Header) -> Option<Self> {
        let forward = (
            Polynomial::from_header(header, "A")?,
            Polynomial::from_header(header, "B")?,
        );

        let inverse = Polynomial::from_header(header, "AP")
            .zip(Polynomial::from_header(header, "BP"))
            .filter(|(a, b)| !a.is_empty() || !b.is_empty());

        Some(Self { forward, inverse })
    }

    /// The corrected pixel offset for a raw one.
    fn correct(&self, (u, v): (f64, f64)) -> (f64, f64) {
        (
            u + self.forward.0.evaluate(u, v),
            v + self.forward.1.evaluate(u, v),
        )
    }

    /// The raw pixel offset for a corrected one.
    fn uncorrect(&self, (u, v): (f64, f64)) -> (f64, f64) {
        match &self.inverse {
            Some((a, b)) => (u + a.evaluate(u, v), v + b.evaluate(u, v)),
            // A header that fitted no inverse still has one; it just has to be
            // found rather than read.
            None => invert_numerically(|point| self.correct(point), (u, v)),
        }
    }
}

/// A polynomial in two variables, as SIP writes it: `A_i_j` is the coefficient
/// of `u^i v^j`.
#[derive(Debug, Clone, PartialEq, Default)]
struct Polynomial {
    /// `(i, j, coefficient)`, in no particular order.
    terms: Vec<(u32, u32, f64)>,
}

impl Polynomial {
    /// Reads the `<name>_ORDER` card and every `<name>_i_j` under it.
    ///
    /// A header that gives coefficients without an order is read anyway, up to
    /// the highest order SIP allows; leaving them out would mean quietly
    /// dropping a correction the header asked for.
    fn from_header(header: &Header, name: &str) -> Option<Self> {
        const MAX_ORDER: u32 = 9;

        let order = number(header, &format!("{}_ORDER", name))
            .map(|order| order.max(0.0).min(MAX_ORDER as f64) as u32);

        let mut terms = Vec::new();
        let limit = order.unwrap_or(MAX_ORDER);

        for i in 0..=limit {
            for j in 0..=(limit - i) {
                if let Some(coefficient) = number(header, &format!("{}_{}_{}", name, i, j))
                    && coefficient != 0.0
                {
                    terms.push((i, j, coefficient));
                }
            }
        }

        if order.is_none() && terms.is_empty() {
            return None;
        }

        Some(Self { terms })
    }

    fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    fn evaluate(&self, u: f64, v: f64) -> f64 {
        self.terms
            .iter()
            .map(|(i, j, coefficient)| coefficient * u.powi(*i as i32) * v.powi(*j as i32))
            .sum()
    }
}

/// The TPV polynomial: `PV1_k` for the first axis and `PV2_k` for the second.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Tpv {
    first: [f64; TPV_TERMS],
    second: [f64; TPV_TERMS],
}

/// How many terms the TPV convention defines, up to seventh order.
const TPV_TERMS: usize = 40;

impl Tpv {
    /// Reads the PVi_k coefficients, or `None` where the header carries none.
    fn from_header(header: &Header) -> Option<Self> {
        let read = |axis: usize| {
            let mut coefficients = [0.0; TPV_TERMS];
            let mut any = false;

            for (term, slot) in coefficients.iter_mut().enumerate() {
                if let Some(value) = number(header, &format!("PV{}_{}", axis, term)) {
                    *slot = value;
                    any = true;
                }
            }

            (coefficients, any)
        };

        let (first, first_present) = read(1);
        let (second, second_present) = read(2);

        if !first_present && !second_present {
            return None;
        }

        // A TPV header that leaves an axis out means the identity there, which
        // is the linear term alone.
        let mut tpv = Self { first, second };
        if !first_present {
            tpv.first[1] = 1.0;
        }
        if !second_present {
            tpv.second[1] = 1.0;
        }

        Some(tpv)
    }

    /// The corrected intermediate coordinates for the raw ones.
    fn correct(&self, (x, y): (f64, f64)) -> (f64, f64) {
        // The second axis uses the same terms with its own coordinate first,
        // which is what makes one table of coefficients serve both.
        (
            evaluate_tpv(&self.first, x, y),
            evaluate_tpv(&self.second, y, x),
        )
    }
}

/// Evaluates the TPV series, whose terms run in the order the convention fixes:
/// the constant, the linear terms, the radial term, then each higher order in
/// turn with its own radial term at odd orders.
fn evaluate_tpv(coefficients: &[f64; TPV_TERMS], x: f64, y: f64) -> f64 {
    let r = x.hypot(y);

    let terms: [f64; TPV_TERMS] = [
        1.0,
        x,
        y,
        r,
        x * x,
        x * y,
        y * y,
        x * x * x,
        x * x * y,
        x * y * y,
        y * y * y,
        r * r * r,
        x.powi(4),
        x.powi(3) * y,
        x * x * y * y,
        x * y.powi(3),
        y.powi(4),
        x.powi(5),
        x.powi(4) * y,
        x.powi(3) * y * y,
        x * x * y.powi(3),
        x * y.powi(4),
        y.powi(5),
        r.powi(5),
        x.powi(6),
        x.powi(5) * y,
        x.powi(4) * y * y,
        x.powi(3) * y.powi(3),
        x * x * y.powi(4),
        x * y.powi(5),
        y.powi(6),
        x.powi(7),
        x.powi(6) * y,
        x.powi(5) * y * y,
        x.powi(4) * y.powi(3),
        x.powi(3) * y.powi(4),
        x * x * y.powi(5),
        x * y.powi(6),
        y.powi(7),
        r.powi(7),
    ];

    coefficients
        .iter()
        .zip(terms)
        .map(|(coefficient, term)| coefficient * term)
        .sum()
}

/// Finds the point `forward` maps onto `target`.
///
/// A distortion is a small correction to the identity, so Newton's method on a
/// numerically differentiated Jacobian settles in a few steps. Where it does not
/// settle — a polynomial fitted far outside the frame it was fitted in can fold
/// the plane over — the best point it reached is returned, which is still the
/// nearest thing to an answer there is.
fn invert_numerically(
    forward: impl Fn((f64, f64)) -> (f64, f64),
    target: (f64, f64),
) -> (f64, f64) {
    /// Close enough that a further step would move the answer by less than a
    /// millionth of a pixel.
    const TOLERANCE: f64 = 1e-12;
    /// The step the Jacobian is measured over, small beside a pixel and large
    /// beside the rounding of a double.
    const STEP: f64 = 1e-6;

    let mut point = target;

    for _ in 0..24 {
        let (fx, fy) = forward(point);
        let residual = (fx - target.0, fy - target.1);

        if residual.0.abs() < TOLERANCE && residual.1.abs() < TOLERANCE {
            break;
        }

        let (dx_x, dx_y) = forward((point.0 + STEP, point.1));
        let (dy_x, dy_y) = forward((point.0, point.1 + STEP));

        let jacobian = [
            [(dx_x - fx) / STEP, (dy_x - fx) / STEP],
            [(dx_y - fy) / STEP, (dy_y - fy) / STEP],
        ];

        let determinant = jacobian[0][0] * jacobian[1][1] - jacobian[0][1] * jacobian[1][0];
        if determinant == 0.0 || !determinant.is_finite() {
            break;
        }

        let step = (
            (jacobian[1][1] * residual.0 - jacobian[0][1] * residual.1) / determinant,
            (jacobian[0][0] * residual.1 - jacobian[1][0] * residual.0) / determinant,
        );

        point = (point.0 - step.0, point.1 - step.1);

        if !point.0.is_finite() || !point.1.is_finite() {
            return target;
        }
    }

    point
}

/// The number a card holds, whether it was written as an integer or a float.
pub(crate) fn number(header: &Header, key: &str) -> Option<f64> {
    match header.card(key)? {
        Value::Float { value, .. } => Some(value),
        Value::Integer { value, .. } => Some(value as f64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Distortion, Polynomial, Sip, Tpv, invert_numerically};
    use crate::header::Header;

    fn sip_header() -> Header {
        let mut header = Header::default();

        header.set_card("A_ORDER", 2_i64).unwrap();
        header.set_card("A_2_0", 1e-5).unwrap();
        header.set_card("B_ORDER", 2_i64).unwrap();
        header.set_card("B_0_2", -2e-5).unwrap();

        header
    }

    #[test]
    fn a_sip_header_is_read_only_when_the_ctype_asks_for_it() {
        let header = sip_header();

        assert!(matches!(
            Distortion::from_header(&header, Some("RA---TAN-SIP")),
            Distortion::Sip(_)
        ));
        // The same coefficients without the CTYPE saying so are not applied: a
        // header that does not claim SIP is not corrected by it.
        assert_eq!(
            Distortion::from_header(&header, Some("RA---TAN")),
            Distortion::None
        );
    }

    #[test]
    fn sip_moves_a_pixel_by_its_polynomial() {
        let sip = Sip::from_header(&sip_header()).expect("the header carries SIP coefficients");

        // u + A_2_0 u^2, v + B_0_2 v^2
        assert_eq!(sip.correct((100.0, 50.0)), (100.1, 49.95));
    }

    #[test]
    fn sip_undoes_itself_even_without_a_fitted_inverse() {
        let sip = Sip::from_header(&sip_header()).expect("the header carries SIP coefficients");

        for point in [(0.0, 0.0), (100.0, 50.0), (-800.0, 600.0)] {
            let corrected = sip.correct(point);
            let back = sip.uncorrect(corrected);

            assert!(
                (back.0 - point.0).abs() < 1e-6 && (back.1 - point.1).abs() < 1e-6,
                "{point:?} came back as {back:?}"
            );
        }
    }

    #[test]
    fn a_fitted_inverse_is_used_when_the_header_has_one() {
        let mut header = sip_header();
        header.set_card("AP_ORDER", 2_i64).unwrap();
        header.set_card("AP_1_0", 0.5).unwrap();
        header.set_card("BP_ORDER", 2_i64).unwrap();

        let sip = Sip::from_header(&header).expect("the header carries SIP coefficients");

        // The fitted inverse is used as given, right or wrong: u + 0.5u.
        assert_eq!(sip.uncorrect((10.0, 0.0)), (15.0, 0.0));
    }

    #[test]
    fn a_polynomial_with_no_coefficients_at_all_is_not_a_polynomial() {
        assert!(Polynomial::from_header(&Header::default(), "A").is_none());
    }

    #[test]
    fn tpv_reads_its_coefficients_and_undoes_itself() {
        let mut header = Header::default();
        header.set_card("PV1_0", 0.0).unwrap();
        header.set_card("PV1_1", 1.0).unwrap();
        header.set_card("PV1_4", 1e-4).unwrap();
        header.set_card("PV2_1", 1.0).unwrap();

        let tpv = Tpv::from_header(&header).expect("the header carries PV coefficients");

        // x + 1e-4 x^2 on the first axis, y untouched on the second.
        let corrected = tpv.correct((2.0, 3.0));
        assert!((corrected.0 - (2.0 + 4e-4)).abs() < 1e-12, "{corrected:?}");
        assert!((corrected.1 - 3.0).abs() < 1e-12, "{corrected:?}");

        let distortion = Distortion::Tpv(Box::new(tpv));
        let back = distortion.uncorrect_intermediate(corrected);
        assert!(
            (back.0 - 2.0).abs() < 1e-9 && (back.1 - 3.0).abs() < 1e-9,
            "{back:?}"
        );
    }

    #[test]
    fn a_map_with_no_inverse_gives_back_what_it_was_asked_about() {
        // A constant map sends everything to one point, so nothing maps back;
        // the answer is the point asked about rather than an infinity.
        let point = invert_numerically(|_| (1.0, 1.0), (5.0, 5.0));

        assert!(point.0.is_finite() && point.1.is_finite(), "{point:?}");
    }
}
