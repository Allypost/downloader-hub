pub mod crop;
pub mod file_extensions;
pub mod file_name;
pub mod media_formats;

use std::sync::Arc;

use once_cell::sync::Lazy;

use crate::fixers::Fixer;

pub type FixerInstance = Arc<dyn Fixer + Send + Sync>;

pub static ALL_FIXERS: Lazy<Vec<FixerInstance>> = Lazy::new(all_fixers);
pub static AVAILABLE_FIXERS: Lazy<Vec<FixerInstance>> = Lazy::new(available_fixers);

fn all_fixers() -> Vec<FixerInstance> {
    vec![
        Arc::new(file_extensions::FileExtension),
        Arc::new(file_name::FileName),
        Arc::new(media_formats::MediaFormats),
        Arc::new(crop::CropBars),
    ]
}

fn available_fixers() -> Vec<FixerInstance> {
    all_fixers().into_iter().filter(|f| f.can_run()).collect()
}
