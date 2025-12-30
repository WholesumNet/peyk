use std::sync::OnceLock;

use indicatif::{
    MultiProgress,
    ProgressBar,
    ProgressStyle
};

static PROGRESS_MANAGER: OnceLock<MultiProgress> = OnceLock::new();

fn get_progress_manager() -> &'static MultiProgress {
    PROGRESS_MANAGER.get_or_init(MultiProgress::new)
}

pub fn new(size: u64) -> ProgressBar {
    let multi = get_progress_manager();
    let pb = multi.add(ProgressBar::new(size));   
    
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta}")
        .unwrap()
        .progress_chars("#>-")
    );    
    pb.set_prefix(String::from("Pulling"));
    pb
}

