use exe::{Buffer, Castable, DebugDirectory, VecPE};
use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

pub struct Windows {
    path: PathBuf,
}

#[derive(Debug)]
pub struct PdbMeta {
    pub name: String,
    pub guid: String,
    pub age: u32,
}

#[repr(C, packed)]
struct DDRaw {
    magic: [u8; 4],
    guid: [u8; 16],
    age: u32,
    name: [u8; 255],
}

unsafe impl Castable for DDRaw {}

const MIN_PDB_NAME_LEN: usize = 4;
const ALLOWED_EXTENSIONS: &[&str] = &["dll", "exe", "sys", "drv", "cpl", "mui", "ocx"];

impl Windows {
    pub fn new(path: PathBuf) -> Self {
        tracing::info!("Creating Windows instance with path: {}", path.display());
        Self { path }
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }

    /// Fetches PDB metadata from files in the System32 directory.
    pub fn fetch_system32_pdbs(&self) -> Result<Vec<PdbMeta>, std::io::Error> {
        tracing::info!("Fetching system32 PDBs from: {}", self.path.display());
        let files = self.get_files_in_system32()?;
        let pdbs = files
            .into_iter()
            .filter_map(|file| match self.get_hash_and_pdb_name(&file) {
                Some(pdb) => Some(pdb),
                None => {
                    tracing::warn!("No PDB found for file: {}", file.display());
                    None
                }
            })
            .collect();
        Ok(pdbs)
    }

    fn get_files_in_system32(&self) -> Result<Vec<PathBuf>, std::io::Error> {
        let system32_path = self.path.join("System32");
        tracing::info!("Listing files in System32: {}", system32_path.display());

        fs::read_dir(system32_path)?
            .filter_map(|entry_result| match entry_result {
                Ok(entry) => {
                    let path = entry.path();
                    if path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(Self::is_allowed_extension)
                        .unwrap_or(false)
                    {
                        tracing::debug!("File accepted: {}", path.display());
                        Some(Ok(path))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(e)),
            })
            .collect()
    }

    fn is_allowed_extension(ext: &str) -> bool {
        ALLOWED_EXTENSIONS
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(ext))
    }

    fn get_hash_and_pdb_name(&self, file: &Path) -> Option<PdbMeta> {
        let image = VecPE::from_file(exe::PEType::Disk, file).ok()?;
        let dir = DebugDirectory::parse(&image).ok()?;
        let dd = image
            .get_ref::<DDRaw>(dir.pointer_to_raw_data.into())
            .ok()?;

        let debug_name = extract_debug_name(&dd.name)?;
        if debug_name.len() < MIN_PDB_NAME_LEN {
            tracing::warn!("PDB name too short in file: {}", file.display());
            return None;
        }
        let age = dd.age;

        tracing::debug!(
            "Debug Name: {}, Debug GUID: {}, Debug Age: {}",
            debug_name,
            encode_guid(&dd.guid),
            age
        );

        Some(PdbMeta {
            name: debug_name,
            guid: encode_guid(&dd.guid),
            age: dd.age,
        })
    }
}

impl PdbMeta {
    /// Downloads the PDB file via a retrying http request.
    pub fn download(&self) -> Option<Vec<u8>> {
        let url = format!(
            "https://msdl.microsoft.com/download/symbols/{}/{}{}/{}",
            self.name, self.guid, self.age, self.name
        );
        tracing::info!("Generated download URL: {}", url);

        let mut attempts = 0;
        let max_attempts = 5;
        let mut delay = Duration::from_secs(1);

        while attempts < max_attempts {
            match reqwest::blocking::get(&url) {
                Ok(response) => {
                    tracing::info!("Successfully fetched data from URL");
                    return Some(response.bytes().unwrap_or_default().to_vec());
                }
                Err(e) => {
                    attempts += 1;
                    tracing::warn!(
                        "Attempt {} failed to fetch data: {}. Retrying in {:?}...",
                        attempts,
                        e,
                        delay
                    );
                    thread::sleep(delay);
                    delay *= 2; // Exponential backoff
                }
            }
        }
        tracing::error!(
            "Failed to fetch data from URL after {} attempts",
            max_attempts
        );
        None
    }
}

/// Extracts a UTF-8 debug name from a null-terminated byte string.
fn extract_debug_name(name: &[u8]) -> Option<String> {
    let name_end = name.iter().position(|&b| b == 0)?;
    String::from_utf8(name[..name_end].to_vec()).ok()
}

/// Encodes a GUID (as found in the binary) into the Microsoft symbol server format.
fn encode_guid(bytes: &[u8; 16]) -> String {
    // Reverse bytes for the first parts per GUID specification.
    hex::encode([
        bytes[3], bytes[2], bytes[1], bytes[0], bytes[5], bytes[4], bytes[7], bytes[6], bytes[8],
        bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    ])
    .to_uppercase()
}
