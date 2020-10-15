use super::error::exit_with_error;
use std::fs;
use std::path::Path;

pub fn create_dir_if_not_exists(output_dir: &Path) {
    if !output_dir.is_dir() {
        if let Err(reason) = fs::create_dir_all(output_dir) {
            exit_with_error("Failed to create directory", reason);
        }
    }
}
