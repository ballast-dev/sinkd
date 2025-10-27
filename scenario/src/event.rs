use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

use log::info;

/// Creates a specified number of files in a folder with a delay between each creation.
pub fn create_files(folder: &Path, num_of_files: usize, delay_secs: f64) {
    fs::create_dir_all(folder).expect("Failed to create folder");
    for i in 0..num_of_files {
        info!("Touching file{i} with delay: {delay_secs}");
        thread::sleep(Duration::from_secs_f64(delay_secs));
        let filepath = folder.join(format!("file{i}"));
        fs::File::create(&filepath)
            .unwrap_or_else(|_| panic!("Failed to create file {}", filepath.display()));
    }
}
