use ::std::env;
use ::std::path::PathBuf;

pub fn get_pitanga_path() -> PathBuf {
    match env::home_dir() {
        Some(home) => home,
        None       => {
            unimplemented!();
        },
    }.join(".pitanga")
}
