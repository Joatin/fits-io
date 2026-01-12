use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::ImageHDU;
use log::LevelFilter;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    TermLogger::init(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    let fits = FsFits::open(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/Light_19 Aurigae_600.0s_Bin1_ISO800_20251117-190134_6.0C_0001.fit"
        )
        .as_ref(),
    )?;

    let primary_hdu = fits.primary_hdu();

    let _image = primary_hdu.read_image(0)?;

    Ok(())
}
