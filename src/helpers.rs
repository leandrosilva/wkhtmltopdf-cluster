use std::fs;
use std::path::Path;
use std::io;

pub fn create_dir_if_not_exists(output_dir: &Path) -> io::Result<()> {
    if output_dir.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(output_dir)
}
