fn main() {
    slint_build::compile("ui/settings.slint").unwrap();

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../icons/OmniSearchTrans.ico");
        res.compile().unwrap();
    }
}
