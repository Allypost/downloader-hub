pub mod crop;
pub mod file_extensions;
pub mod file_name;
pub mod media_formats;
pub mod split_scenes;

use std::sync::Arc;

use once_cell::sync::Lazy;

use crate::Fixer;

pub type FixerInstance = Arc<dyn Fixer>;
pub static DEFAULT_FIXERS: Lazy<Vec<FixerInstance>> = Lazy::new(|| {
    vec![
        Arc::new(file_extensions::FileExtension),
        Arc::new(file_name::FileName),
        Arc::new(media_formats::MediaFormats),
        Arc::new(crop::CropBars),
    ]
});

pub fn default_fixers() -> Vec<FixerInstance> {
    DEFAULT_FIXERS.clone()
}

pub static ALL_FIXERS: Lazy<Vec<FixerInstance>> = Lazy::new(|| {
    let mut defaults = DEFAULT_FIXERS.clone();
    let rest: Vec<FixerInstance> = vec![
        Arc::new(split_scenes::SplitScenes),
    ];
    defaults.extend(rest);

    defaults
});
