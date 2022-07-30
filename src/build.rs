use std::{
    env, fmt, fs,
    fs::File,
    path::{Path, PathBuf, MAIN_SEPARATOR}
};

const DRIVER_VERSION: &str = "1.11.0";
// const DRIVER_VERSION: &str = "1.12.2";
const NEXT: &str = "";

fn main() {
    println!("cargo:rerun-if-env-changed=DRIVER_VERSION");
    println!("cargo:rerun-if-changed=src/build.rs");
    println!("cargo:rustc-env=SEP={}", MAIN_SEPARATOR);
    let out_dir: PathBuf = env::var_os("OUT_DIR").unwrap().into();
    let dest = out_dir.join("driver.zip");
    fs::remove_file(dest.clone()).or::<()>(Ok(())).unwrap();
    let platform = PlaywrightPlatform::default();
    let url = url(platform);
    fs::write(out_dir.join("platform"), platform.to_string()).unwrap();
    fs::write(out_dir.join("pleywright_link"), url.clone()).unwrap();
    download(&url, &dest);
}

fn remove_cached_file() {
    let cache_dir: &Path = "/tmp/build-playwright-rust".as_ref();
    let cached = cache_dir.join("driver.zip");
    fs::remove_file(cached).or::<()>(Ok(())).unwrap();
}

#[cfg(all(not(feature = "only-for-docs-rs")))]
fn download(url: &str, dest: &Path) {
    let mut resp = reqwest::blocking::get(url).unwrap();
    let mut dest = File::create(dest).unwrap();
    resp.copy_to(&mut dest).unwrap();
}

fn size(p: &Path) -> u64 {
    let maybe_metadata = p.metadata().ok();
    let size = maybe_metadata
        .as_ref()
        .map(fs::Metadata::len)
        .unwrap_or_default();
    size
}

fn check_size(p: &Path) {
    assert!(size(p) > 10_000_000, "file size is smaller than the driver");
}

// No network access
#[cfg(feature = "only-for-docs-rs")]
fn download(_url: &str, dest: &Path) { File::create(dest).unwrap(); }

fn url(platform: PlaywrightPlatform) -> String {
    let driver_version = env::var("DRIVER_VERSION").unwrap_or(String::from(DRIVER_VERSION));
    format!(
        "https://playwright.azureedge.net/builds/driver{}/playwright-{}-{}.zip",
        NEXT, driver_version, platform
    )
}

#[derive(Clone, Copy)]
enum PlaywrightPlatform {
    Linux,
    Win32,
    Win32x64,
    Mac
}

impl fmt::Display for PlaywrightPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Linux => write!(f, "linux"),
            Self::Win32 => write!(f, "win32"),
            Self::Win32x64 => write!(f, "win32_x64"),
            Self::Mac => write!(f, "mac")
        }
    }
}

impl Default for PlaywrightPlatform {
    fn default() -> Self {
        match env::var("CARGO_CFG_TARGET_OS").as_deref() {
            Ok("linux") => return PlaywrightPlatform::Linux,
            Ok("macos") => return PlaywrightPlatform::Mac,
            _ => ()
        };
        if env::var("CARGO_CFG_WINDOWS").is_ok() {
            if env::var("CARGO_CFG_TARGET_POINTER_WIDTH").as_deref() == Ok("64") {
                PlaywrightPlatform::Win32x64
            } else {
                PlaywrightPlatform::Win32
            }
        } else if env::var("CARGO_CFG_UNIX").is_ok() {
            PlaywrightPlatform::Linux
        } else {
            panic!("Unsupported plaform");
        }
    }
}
