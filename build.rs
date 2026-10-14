fn main() {
    fastframe_i18n::build::compile_catalogs("assets/i18n");
    println!("cargo:rerun-if-changed=assets/icon.ico");

    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        if let Err(error) = resource.compile() {
            println!("cargo:warning=could not embed the exe icon: {error}");
        }
    }
}
