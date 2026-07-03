fn main() {
    slint_build::compile("ui/settings.slint").unwrap();

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../icons/OmniSearchTrans.ico");
        res.set_language(0x0409); // U.S. English
        res.set("FileDescription", "OmniSearch Launcher");
        res.set("ProductName", "OmniSearch");
        res.set("OriginalFilename", "omnisearch.exe");
        res.set("CompanyName", "Pranshul Soni");
        res.set("LegalCopyright", "Copyright (c) 2026 Pranshul Soni");
        res.compile().unwrap();
    }
}
