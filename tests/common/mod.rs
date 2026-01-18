use std::fs::File;
use std::path::PathBuf;

pub fn create_dummy_file(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(name);
    File::create(&path).unwrap();
    path
}

pub fn remove_file(path: PathBuf) {
    let _ = std::fs::remove_file(path);
}
