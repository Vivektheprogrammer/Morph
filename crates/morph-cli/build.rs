fn main() {
    println!("cargo:rerun-if-changed=../../assets/logo.ico");
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let icon_path = std::path::Path::new(&manifest_dir).join("../../assets/logo.ico");
        let mut res = winres::WindowsResource::new();
        res.set_icon(icon_path.to_str().unwrap());
        res.compile().expect("Failed to compile Windows resource with logo icon");
    }
}
