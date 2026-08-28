use crate::util::ReadSeek;
use std::fs::OpenOptions;
use std::io;
use std::path::Path;

/// Returns true when `path` names a gzip-compressed file.
///
/// Note that `Path::ends_with` matches whole path *components*, so it never
/// matches a file extension — `Path::new("a.fits.gz").ends_with(".gz")` is
/// `false`. The extension has to be compared explicitly.
#[cfg(feature = "gzip")]
fn is_gzipped(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gz"))
}

pub fn open_fits_file(path: &Path) -> Result<Box<dyn ReadSeek>, io::Error> {
    let file = OpenOptions::new().read(true).open(path)?;

    #[cfg(feature = "gzip")]
    if is_gzipped(path) {
        use std::io::Read;

        let mut decoder = flate2::read::GzDecoder::new(file);

        let mut data = Vec::new();
        decoder.read_to_end(&mut data)?;

        return Ok(Box::new(io::Cursor::new(data)));
    }

    Ok(Box::new(file))
}

#[cfg(all(test, feature = "gzip"))]
mod tests {
    use super::is_gzipped;
    use std::path::Path;

    #[test]
    fn gzipped_fits_files_are_detected() {
        assert!(is_gzipped(Path::new("image.fits.gz")));
        assert!(is_gzipped(Path::new("image.fit.gz")));
        assert!(is_gzipped(Path::new("image.fts.gz")));
        assert!(is_gzipped(Path::new("/some/dir/image.fits.gz")));
        assert!(is_gzipped(Path::new("./relative/image.fits.GZ")));
    }

    #[test]
    fn uncompressed_fits_files_are_not_detected_as_gzipped() {
        assert!(!is_gzipped(Path::new("image.fits")));
        assert!(!is_gzipped(Path::new("image.fit")));
        assert!(!is_gzipped(Path::new("/some/dir/image.fts")));
    }

    #[test]
    fn a_gz_directory_component_is_not_a_gz_extension() {
        assert!(!is_gzipped(Path::new(".gz/image.fits")));
        assert!(!is_gzipped(Path::new("/archive/.gz/image.fits")));
    }
}
