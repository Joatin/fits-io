use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::{BinTableHDU, ExtensionHDU};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GaiaCatalogEntry {
    pub source_id: i64,
    pub ra: f64,
    pub dec: f64,
    pub phot_g_mean_mag: f32,
}

#[test]
pub fn open_bin_table_should_work() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let fits = FsFits::open("tests/gaia-dr3-mag-gt-12.fits".as_ref())?;

    if let Some(ExtensionHDU::BinTable(bin_table)) = fits.extension_hdu(0) {
        let rows: Vec<GaiaCatalogEntry> = bin_table.read_rows()?;

        assert_eq!(rows.len(), 2482633);
    } else {
        panic!("No extension 0 found");
    }

    Ok(())
}
