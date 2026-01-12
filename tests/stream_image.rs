use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::ImageHDU;
use futures::StreamExt;

#[tokio::test]
async fn it_stream_image() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let fits = FsFits::open_async(
        "tests/Light_19 Aurigae_600.0s_Bin1_ISO800_20251117-190134_6.0C_0001.fit".as_ref(),
    )
    .await?;
    let primary_hdu = fits.primary_hdu();

    let stream = primary_hdu.stream_normalised_image(0)?.unwrap();

    let _sum = stream
        .fold(0.0, |mut acc, (_x, _y, value)| async move {
            acc += value;
            acc
        })
        .await;

    Ok(())
}
