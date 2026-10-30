fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/iconApp.ico");
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/iconApp.ico");
        resource.set("ProductName", "Small Demo Manager");
        resource.set("FileDescription", "Native CS2 Demo Manager");
        resource.set("LegalCopyright", "Copyright (c) 2026 darkcr0w. All rights reserved. Personal use permitted. Original project and third-party notices apply.");
        resource
            .compile()
            .expect("compile Windows application resources");
    }
}
