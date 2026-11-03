//! A file or folder of a project moved to the Trash, as the Finder does:
//! taken back from there, not deleted.

use std::path::Path;

#[cfg(target_os = "macos")]
pub fn trash(path: &Path) -> Result<(), String> {
    use objc2_foundation::{NSFileManager, NSString, NSURL};
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    NSFileManager::defaultManager()
        .trashItemAtURL_resultingItemURL_error(&url, None)
        .map_err(|e| format!("{}: {}", path.display(), e.localizedDescription()))
}

#[cfg(not(target_os = "macos"))]
pub fn trash(path: &Path) -> Result<(), String> {
    Err(format!("{}: the Trash on macOS only", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Puts a file in the Trash of whoever runs it: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn a_file_goes_to_the_trash() {
        let path = std::env::temp_dir().join("papers-review-trash-test.txt");
        std::fs::write(&path, "taken back from the Trash").unwrap();
        trash(&path).unwrap();
        assert!(!path.exists());
        assert!(trash(&path).is_err());
    }
}
