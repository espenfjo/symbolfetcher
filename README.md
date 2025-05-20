# symbolfetcher

**symbolfetcher** is a Rust tool for discovering and downloading Windows PDB (Program Database) symbol files from a Windows installation directory. It scans the `System32` folder for PE files (DLL, EXE, SYS, etc.), extracts their PDB metadata, and downloads the corresponding symbol files from the Microsoft Symbol Server.

## Features

- Scans a Windows installation's `System32` directory for PE files.
- Extracts PDB name, GUID, and age from each file's debug directory.
- Downloads matching PDB files from the Microsoft Symbol Server.
- Retries downloads with exponential backoff.
- Structured logging with `tracing`.

## Usage

```sh
cargo run -- /path/to/windows/installation
```

- Replace `/path/to/windows/installation` with the path to your Windows directory (should contain a `System32` folder).

Downloaded PDBs are saved in the `pdbs/` directory, organized by name, GUID, and age in the same way WinDBG or a symbol server expectes them.

## Example

```sh
cargo run -- /mnt/windows
```

## Dependencies

- [clap](https://crates.io/crates/clap) for CLI parsing
- [exe](https://crates.io/crates/exe) for PE file parsing
- [hex](https://crates.io/crates/hex) for GUID encoding
- [reqwest](https://crates.io/crates/reqwest) for HTTP requests
- [tracing](https://crates.io/crates/tracing) for logging

## Logging

Set the log level using the `RUST_LOG` environment variable:

```sh
RUST_LOG=symbolfetcher=debug cargo run -- /mnt/windows
```

## License

MIT

---

*This project is a work in progress. Contributions and issues are welcome!*
