//! Fonts are private to this process, never installed over the desktop fonts.
use std::{ffi::CString, path::PathBuf};
#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcConfigGetCurrent() -> *mut std::ffi::c_void;
    fn FcConfigAppFontAddFile(config: *mut std::ffi::c_void, file: *const u8) -> i32;
}
pub fn directory() -> PathBuf {
    if let Some(path) = std::env::var_os("BIBBIA_ASSETS_DIR") {
        return path.into();
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(prefix) = exe.parent().and_then(|p| p.parent())
    {
        let path = prefix.join("share/bibbia");
        if path.is_dir() {
            return path;
        }
    }
    #[cfg(debug_assertions)]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets")
    }
    #[cfg(not(debug_assertions))]
    {
        PathBuf::from("/usr/share/bibbia")
    }
}
pub fn load_fonts() {
    for name in ["AdwaitaSans-Regular.ttf", "SourceSerif4-Regular.ttf"] {
        let path = directory().join("fonts").join(name);
        if let Ok(path) = CString::new(path.as_os_str().as_encoded_bytes()) {
            // Fontconfig copies the path; both the config and GTK live on the
            // main thread, and the C string is valid for the complete call.
            unsafe {
                if FcConfigAppFontAddFile(FcConfigGetCurrent(), path.as_ptr().cast()) == 0 {
                    eprintln!("Bibbia: font unavailable: {name}");
                }
            }
        }
    }
}
