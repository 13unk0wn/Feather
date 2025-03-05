fn main() {
    if pkg_config::probe_library("mpv").is_err() {
        println!("cargo:warning=Could not find mpv via pkg-config. Make sure it is installed");
    }
}
