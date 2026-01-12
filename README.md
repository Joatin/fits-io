# FitsIo

A safe, ergonomic, and pure-Rust library for reading and writing FITS (Flexible Image Transport System) files, inspired by CFITSIO.

This crate supports no_std environments, optional async I/O with Tokio, and structured access to FITS headers, images, and tables — without any C dependencies.

Designed for astronomy, astrophotography, embedded systems, and scientific pipelines where portability and safety matter.

## Features

* 📦 Pure Rust implementation (no CFITSIO, no C bindings)
* 🚫 no_std compatible
* ⚡ Async I/O with Tokio (enabled by default)
* 🧩 Support for Primary HDUs and extensions
* 🖼️ Image HDUs
* 📊 ASCII tables and binary tables
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

**no_std** mode
```toml
[dependencies]
fits-io = { version = "0.1", default-features = false }
```

## Design Goals

* **Safety** — eliminate undefined behavior and unsafe FFI
* **Portability** — run anywhere Rust runs
* **Ergonomics** — minimal boilerplate
* **Performance** — streaming-friendly, low overhead
* **Familiarity** — CFITSIO-inspired, Rust-native

## Supported FITS Features

| Feature           | Status |
|-------------------|--------|
| Primary HDU       | ✅      |
| Image HDU	        | ✅      |
| Binary Tables     | ✅      |
| ASCII Tables	     | ✅      | 
| Header read/write | ✅      | 
| Compression       | ✅      |
| Streaming I/O	    | 🚧     |    
| WCS helpers	      | 🚧     |

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
