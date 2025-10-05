# Dicomancer

Dicomancer is a cross-platform desktop app for inspecting DICOM studies. Built in Rust with the [`iced`](https://github.com/iced-rs/iced) GUI toolkit, it provides a polished view over DICOM metadata while keeping the tooling lightweight and approachable.

## Features

- **Import multiple files** – Select one or many DICOM files and browse them all in a single session.
- **Toggleable navigation** – Switch between a UID tree grouped by patient/study/series hierarchy or a simple file-browser list.
- **Metadata inspector** – View every tag with its alias, VR, and value in a readable table with word wrapping.
- **Pixel preview** – Render the first frame (when available) so you can sanity-check images alongside metadata.


## Getting Started

### Prerequisites

- Rust toolchain (1.75 or newer recommended)
- Cargo (bundled with Rust)

### Clone & Run

```bash
git clone https://github.com/your-user/dicomancer.git
cd dicomancer

# (Optional) lint and format
cargo fmt
cargo clippy --all-targets --all-features

# Launch the app
cargo run
```

Sample data is available in `data/`—try `CT_small.dcm` to get an immediate feel for the UI.

## Code Layout

```
src/
├── app.rs              # Application state & update loop
├── main.rs             # Thin entry point calling `app::run`
├── components/         # Reusable widgets (e.g., segmented toggle)
├── views/              # UI panels: tree browser, metadata panel, image viewer
├── model/              # DICOM models, loader, tree state
├── utils/              # Helpers for formatting values, tags
└── image_pipeline.rs   # Frame extraction and image rendering pipeline
```

## Development

- `cargo fmt` – enforce formatting
- `cargo clippy --all-targets --all-features` – catch common mistakes
- `cargo run` – start the development build
- `cargo build --release` – produce an optimized binary

Logging is controlled through `RUST_LOG`; set `RUST_LOG=info cargo run` to see basic diagnostics.

## Contributing

Issues and pull requests are welcome! Please run formatting and clippy before submitting changes and describe the behavior you’re proposing or fixing.

## License

This project is currently unlicensed. Add a license file (e.g., `LICENSE` or `LICENSE-MIT`) before distributing binaries.
