# pkgextract

Simple Rust CLI for extracting PlayStation 4 `.pkg` files.

## Features

- Main command: `pkgextract <file.pkg>`
- Automatically creates an output folder using the package filename (without extension)
- Extracts PKG contents into that folder
- Shows a terminal progress bar during extraction

## Installation

### With `cargo`

```bash
cargo install --path .
```

### Local build

```bash
cargo build --release
```

Generated binary:

```text
target/release/pkgextract
```

## Usage

```bash
pkgextract MY_GAME.pkg
```

Options:

- `-o, --output <DIR>`: set output directory
- `-f, --overwrite`: overwrite existing output directory

Examples:

```bash
pkgextract CUSA00000.pkg
pkgextract CUSA00000.pkg -o extracted/CUSA00000
pkgextract CUSA00000.pkg -f
```

## GitHub Releases

When a GitHub Release is published, the workflow builds and uploads binaries for:

- Windows x64
- Windows ARM64
- Linux x64
- Linux ARM64
- macOS Intel
- macOS Apple Silicon

## Notice

Use this software only with files you own and are legally allowed to process.

## License

This project is released under [The Unlicense](LICENSE) (public domain).
