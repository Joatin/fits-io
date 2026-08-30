use crate::header::Header;

/// One group of a random-groups HDU.
///
/// The convention predates image extensions: rather than one array, the primary
/// HDU holds a run of groups, each carrying a few parameters — a time, a
/// baseline, a coordinate — followed by the array those parameters describe. It
/// is mostly met in older radio interferometry data.
#[derive(Debug, Clone)]
pub struct Group {
    parameters: Vec<f64>,
    names: Vec<String>,
    data: Vec<f64>,
}

impl Group {
    pub(crate) fn new(parameters: Vec<f64>, names: Vec<String>, data: Vec<f64>) -> Self {
        Self {
            parameters,
            names,
            data,
        }
    }

    /// This group's parameters, in physical units.
    ///
    /// PSCALn and PZEROn have already been applied, so these are the values the
    /// parameters stand for rather than the numbers stored.
    pub fn parameters(&self) -> &[f64] {
        &self.parameters
    }

    /// The parameter named by a PTYPEn card.
    ///
    /// The standard allows a parameter to be split across several entries that
    /// share a name, so that a value needing more precision than the array's
    /// type offers can be summed from its parts. Those parts are added here.
    pub fn parameter(&self, name: &str) -> Option<f64> {
        let mut total = None;

        for (index, parameter_name) in self.names.iter().enumerate() {
            if parameter_name == name {
                total = Some(total.unwrap_or(0.0) + self.parameters.get(index).copied()?);
            }
        }

        total
    }

    /// The names of this group's parameters, from the PTYPEn cards.
    pub fn parameter_names(&self) -> &[String] {
        &self.names
    }

    /// This group's array, in physical units.
    pub fn data(&self) -> &[f64] {
        &self.data
    }
}

/// Reads the parameters and array of one group out of its bytes.
pub(crate) fn decode_group(header: &Header, bytes: &[u8]) -> Option<Group> {
    let bitpix = header.bitpix()?;
    let width = bitpix.byte_size();

    let count = header.pcount().unwrap_or(0).max(0) as usize;
    let zero = header.bzero_or_default();
    let scale = header.bscale_or_default();

    let mut parameters = Vec::with_capacity(count);
    let mut names = Vec::with_capacity(count);

    for index in 0..count {
        let raw = bitpix.read_be(bytes.get(index * width..)?)?;

        // A parameter carries its own scaling, separate from the array's.
        let parameter_scale = header.parameter_scaling_factor(index).unwrap_or(1.0);
        let parameter_zero = header.parameter_scaling_zero_point(index).unwrap_or(0.0);

        parameters.push(parameter_zero + parameter_scale * raw);
        names.push(header.parameter_type(index).unwrap_or_default().to_string());
    }

    let array = bytes.get(count * width..)?;
    let data = array
        .chunks_exact(width)
        .filter_map(|raw| bitpix.read_be(raw))
        .map(|raw| zero + scale * raw)
        .collect();

    Some(Group::new(parameters, names, data))
}
