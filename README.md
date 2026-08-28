# FitsIo

A safe, ergonomic, and pure-Rust library for reading FITS (Flexible Image Transport System) files, inspired by CFITSIO. Writing is planned; see the feature table below.

This crate offers optional async I/O with Tokio and structured access to FITS headers, images, and tables — without any C dependencies.

Designed for astronomy, astrophotography, and scientific pipelines where portability and safety matter.

## Features

* 📦 Pure Rust implementation (no CFITSIO, no C bindings)
* ⚡ Async I/O with Tokio (enabled by default)
* 🧩 Support for Primary HDUs and extensions
* 🖼️ Image HDUs
* 📊 Binary tables, with optional `serde` deserialisation into your own structs
* 🧠 Typed access to FITS header keywords
* 🚀 Streaming and memory-efficient reads
* 🛡️ Idiomatic error handling with Result
* 🔁 CFITSIO-inspired API, redesigned for Rust

## Installation

Add the crate to your Cargo.toml:
```toml
[dependencies]
fits-io = "0.1"
```

### Feature flags

`default-features = false` gives you header, image and table parsing over
in-memory data, with no filesystem, async or threading support.

| Feature | Default | Effect                                                    |
|---------|---------|-----------------------------------------------------------|
| `fs`    | ✅      | Read FITS files from the filesystem via `FsFits`          |
| `gzip`  | ✅      | Transparently decompress `.fits.gz` files (implies `fs`)  |
| `tokio` | ✅      | Async open and streaming reads                            |
| `rayon` | ✅      | Parallel binary-table row decoding                        |
| `serde` |         | Deserialize binary-table rows into your own structs       |

## Design Goals

* **Safety** — eliminate undefined behavior and unsafe FFI
* **Portability** — run anywhere Rust runs
* **Ergonomics** — minimal boilerplate
* **Performance** — streaming-friendly, low overhead
* **Familiarity** — CFITSIO-inspired, Rust-native

## Supported FITS Features

| Feature                    | Status | Notes                                              |
|----------------------------|--------|----------------------------------------------------|
| Primary HDU                | ✅      |                                                    |
| Extension HDUs             | ✅      |                                                    |
| Image HDU                  | ✅      | Including 3-axis cubes                             |
| Binary tables              | ✅      | Read; `serde` deserialises rows into your structs  |
| ASCII tables               | 🚧     | Detected, but reading is not implemented           |
| Header read                | ✅      |                                                    |
| Gzip decompression         | ✅      | `.fits.gz` and friends, via the `gzip` feature     |
| Streaming image reads      | ✅      | `stream_normalised_image`, via the `tokio` feature |
| Streaming table rows       | 🚧     |                                                    |
| Writing (headers and data) | 🚧     | Every write entry point returns an error today     |
| Variable-length array columns | 🚧  | TFORMn `P` and `Q` descriptors                     |
| Complex columns            | 🚧     | TFORMn `C` and `M`                                 |
| WCS helpers                | 🚧     |                                                    |

## License

Licensed under either of:

* Apache License, Version 2.0
* MIT License

at your option.

## Contributing

Issues, discussions, and pull requests are welcome.
Please open an issue for large changes or new features.

## Acknowledgements

Inspired by CFITSIO and the FITS standard maintained by NASA/HEASARC.

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
