pub fn is_fits_file(path: &std::path::Path) -> bool {
    if let Some(file_name) = path.file_name() {
        if let Some(file_name) = file_name.to_str() {
            file_name.ends_with(".fit")
                || file_name.ends_with(".fits")
                || file_name.ends_with(".fts")
                || file_name.ends_with(".fit.gz")
                || file_name.ends_with(".fits.gz")
                || file_name.ends_with(".fts.gz")
        } else {
            false
        }
    } else {
        false
    }
}
