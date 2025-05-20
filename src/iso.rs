use cdfs::ISO9660;
use std::fs::File;
use tracing::info;

pub struct Iso(pub ISO9660<File>);
impl Iso {
    pub fn new(_path: std::path::PathBuf) -> Result<Self, std::io::Error> {
        info!("Opening ISO file: {}", _path.display());
        let reader = File::open(_path)?;
        let iso = ISO9660::new(reader).expect("Failed to read ISO file");

        Ok(Self(iso))
    }
}
