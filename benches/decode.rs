//! Measures the work that dominates reading a real file, so that a change which
//! claims to be faster can be checked rather than assumed.
//!
//! Run with `cargo bench --features fs,serde,rayon`.

use fits_io::Fits;
use fits_io::bin_table::from_bin_table;
use fits_io::fs::FsFits;
use fits_io::hdu::{BinTableHDU, ExtensionHDU};
use serde::Deserialize;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Deserialize)]
struct Source {
    // Read for its side effect on the deserializer, not for its value.
    #[serde(rename = "source_id")]
    #[allow(dead_code)]
    id: i64,
}

fn fixture() -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("gaia-dr3-mag-gt-12.fits");

    match std::fs::metadata(&path) {
        Ok(metadata) if metadata.len() > 4096 => Some(path),
        _ => None,
    }
}

/// Times `run` a few times over and reports the fastest, which is the least
/// polluted by whatever else the machine was doing.
fn time(name: &str, mut run: impl FnMut()) {
    const RUNS: usize = 5;

    let best = (0..RUNS)
        .map(|_| {
            let at = Instant::now();
            run();
            at.elapsed()
        })
        .min()
        .expect("at least one run");

    println!("{name:<28} {best:>12.3?}");
}

fn main() {
    let Some(path) = fixture() else {
        eprintln!("skipping: the fixture is not present (run `git lfs pull`)");
        return;
    };

    println!(
        "{}",
        if cfg!(feature = "rayon") {
            "rayon: on"
        } else {
            "rayon: off"
        }
    );

    time("open", || {
        black_box(FsFits::open(&path).expect("the fixture opens"));
    });

    let fits = FsFits::open(&path).expect("the fixture opens");
    let Some(ExtensionHDU::BinTable(hdu)) = fits.extension_hdu(0) else {
        eprintln!("skipping: the fixture's first extension is not a table");
        return;
    };

    time("read_table", || {
        black_box(hdu.read_table().expect("the table reads"));
    });

    let table = hdu.read_table().expect("the table reads");
    println!("{:<28} {} rows", "size", table.len());

    time("decode every column", || {
        for row in table.rows() {
            for column in 0..table.field_definitions().len() {
                black_box(row.get_at(column).expect("the column decodes"));
            }
        }
    });

    time("deserialize into a struct", || {
        let rows: Vec<Source> = from_bin_table(&table).expect("the rows deserialize");
        black_box(rows.len());
    });
}
