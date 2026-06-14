use std::path::PathBuf;

fn main() {
    #[cfg(not(target_os = "windows"))]
    build_libemf2svg();

    tauri_build::build()
}

#[cfg(not(target_os = "windows"))]
fn build_libemf2svg() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let deps_dir = manifest_dir.parent().unwrap().join("deps");
    let libemf2svg_dir = deps_dir.join("libemf2svg");
    let build_dir = libemf2svg_dir.join("build");

    if libemf2svg_dir.exists() {
        let cmake = find_cmake();
        std::env::set_var("CMAKE", &cmake);

        let mut cfg = cmake::Config::new(&libemf2svg_dir);
        cfg.out_dir(&build_dir)
            .define("LONLY", "ON")
            .generator("Unix Makefiles");

        let dst = cfg.build();
        println!("cargo:rustc-link-search=native={}", dst.join("lib").display());
        println!("cargo:rustc-link-lib=dylib=emf2svg");

        // Add rpath so the dynamic library is found at runtime.
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/../lib");
    }
}

fn find_cmake() -> String {
    // Prefer pip-installed cmake on macOS.
    let pip_cmake = PathBuf::from(env!("HOME"))
        .join("Library/Python/3.9/bin/cmake");
    if pip_cmake.exists() {
        return pip_cmake.to_string_lossy().to_string();
    }
    "cmake".to_string()
}
