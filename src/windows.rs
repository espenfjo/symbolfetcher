use exe::{Buffer, Castable, DebugDirectory, VecPE};

pub struct Windows {
    pub path: std::path::PathBuf,
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

fn extract_debug_name(name: &[u8]) -> Option<String> {
    let name_end = name.iter().position(|&b| b == 0)?;
    String::from_utf8(name[..name_end].to_vec()).ok()
}

fn encode_guid(wg: &[u8; 16]) -> String {
    hex::encode([
        wg[3], wg[2], wg[1], wg[0], wg[5], wg[4], wg[7], wg[6], wg[8], wg[9], wg[10], wg[11],
        wg[12], wg[13], wg[14], wg[15],
    ])
    .to_uppercase()
}

impl PdbMeta {
    pub fn download(&self) -> Option<Vec<u8>> {
        let url = format!(
            "https://msdl.microsoft.com/download/symbols/{}/{}{}/{}",
            self.name, self.guid, self.age, self.name
        );
        tracing::info!("Generated download URL: {}", url);

        let mut attempts = 0;
        let max_attempts = 5;
        let mut delay = std::time::Duration::from_secs(1);
        while attempts < max_attempts {
            match reqwest::blocking::get(&url) {
                Ok(response) => {
                    tracing::info!("Successfully fetched data from URL");
                    return Some(response.bytes().unwrap_or_default().to_vec());
                }
                Err(e) => {
                    tracing::warn!(
                        "Attempt {} failed to fetch data: {}. Retrying in {:?}...",
                        attempts + 1,
                        e,
                        delay
                    );
                    std::thread::sleep(delay);
                    delay *= 2; // Exponential backoff
                    attempts += 1;
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

impl Windows {
    pub fn new(path: std::path::PathBuf) -> Self {
        tracing::info!("Creating Windows instance with path: {}", path.display());
        Self { path }
    }

    pub fn get_path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn fetch_system32_pdbs(&self) -> Result<Vec<PdbMeta>, std::io::Error> {
        tracing::info!("Fetching system32 PDBs from: {}", self.path.display());
        let files = self.get_files_in_system32()?;
        let hnp = self.get_hash_and_pdb_names(&files);
        Ok(hnp)
    }
    fn get_hash_and_pdb_names(&self, files: &[std::path::PathBuf]) -> Vec<PdbMeta> {
        tracing::info!("Extracting hash and PDB names from files");
        let mut hnps = Vec::new();
        for file in files {
            let hnp = self.get_hash_and_pdb_name(file);
            if hnp.is_none() {
                tracing::warn!("No PDB found for file: {}", file.display());
                continue;
            }
            hnps.push(hnp.unwrap());
        }
        hnps
    }

    fn get_hash_and_pdb_name(&self, file: &std::path::Path) -> Option<PdbMeta> {
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

        let debug_guid = encode_guid(&dd.guid);
        let debug_age = dd.age;

        tracing::debug!(
            "Debug Name: {}, Debug GUID: {}, Debug Age: {}",
            debug_name,
            debug_guid,
            debug_age
        );

        Some(PdbMeta {
            name: debug_name,
            guid: debug_guid,
            age: debug_age,
        })
    }

    fn get_files_in_system32(&self) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
        let system32_path = self.path.join("System32");
        tracing::info!("Listing files in System32: {}", system32_path.display());

        std::fs::read_dir(system32_path)?
            .map(
                |entry_result| -> Result<Option<std::path::PathBuf>, std::io::Error> {
                    let entry = entry_result?;
                    let path = entry.path();
                    if path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| {
                            ext.eq_ignore_ascii_case("dll")
                                || ext.eq_ignore_ascii_case("exe")
                                || ext.eq_ignore_ascii_case("sys")
                                || ext.eq_ignore_ascii_case("drv")
                                || ext.eq_ignore_ascii_case("cpl")
                                || ext.eq_ignore_ascii_case("mui")
                                || ext.eq_ignore_ascii_case("ocx")
                        })
                        .unwrap_or(false)
                    {
                        tracing::debug!("DLL file found: {}", path.display());
                        Ok(Some(path))
                    } else {
                        Ok(None)
                    }
                },
            )
            .collect::<Result<Vec<Option<std::path::PathBuf>>, std::io::Error>>()
            .map(|opts| opts.into_iter().flatten().collect())
    }
}
