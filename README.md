# ps4-pkg-extractor

Simple Rust CLI for extracting PlayStation 4 `.pkg` files.

## Features

- Main command: `pkgextract <file.pkg>`
- Automatically creates an output folder using the package filename (without extension)
- Extracts PKG contents into that folder
- Shows a terminal progress bar during extraction

## Installation

Download the latest binary for your platform from the GitHub Releases page:

- [Releases](https://github.com/paulomanrique/ps4-pkg-extractor/releases)

Each release asset is a `.zip` file containing a binary named `pkgextract`
(`pkgextract.exe` on Windows).

After downloading, extract the archive.

On Unix systems, make the binary executable if needed:

```bash
chmod +x pkgextract
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

## License

This project is released under [The Unlicense](LICENSE) (public domain).
