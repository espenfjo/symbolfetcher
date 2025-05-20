use clap::Parser as _;
use std::{fs, path::PathBuf};
use tracing::{debug, error, warn};

pub mod windows;
#[derive(clap::Parser, Debug)]
struct Cli {
    /// Path to the windows installation
    folder: PathBuf,
}
fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("symbolfetch=debug")
        .with_file(true)
        .with_line_number(true)
        .init();
    let cli = Cli::parse();
    let windows = windows::Windows::new(cli.folder);
    let pdbs = windows.fetch_system32_pdbs().unwrap();
    for pdb in pdbs {
        debug!("PDB: {:?}", pdb);
        let pdb_folder = PathBuf::from(format!("pdbs/{}/{}{}/", pdb.name, pdb.guid, pdb.age));
        let pdb_path = pdb_folder.join(&pdb.name);
        if pdb_path.exists() {
            warn!("PDB already exists: {:?}", pdb_path);
            continue;
        }
        let data = pdb.download();
        if data.is_none() {
            error!("Failed to download PDB: {:?}", pdb);
            continue;
        }
        if !pdb_folder.exists() {
            fs::create_dir_all(&pdb_folder)
                .unwrap_or_else(|_| panic!("Failed to create directory for PDB: {}", pdb.name));
        }
        fs::write(pdb_path, data.unwrap()).expect("Failed to write PDB data to file");
    }

    // let iso = Iso::new(cli.iso).expect("Failed to open ISO file");
    // let wim = wim::Wim::new(&iso, cli.image).expect("Failed to open WIM image from ISO");
}
