extern crate winres;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();

        #[cfg(all(not(feature = "kiwami2"), not(feature = "kiwami")))]
        let name = "yakuza0";

        #[cfg(feature = "kiwami")]
        let name = "kiwami";

        #[cfg(feature = "kiwami2")]
        let name = "kiwami2";

        cc::Build::new()
            .file(&format!("src\\asm\\{}.asm", name).to_string())
            .compile(&format!("{}-asm", name).to_string());
        println!("cargo:rerun-if-changed=src\\asm\\{}.asm", name);

        res.set_icon(&format!("assets\\{}.ico", name).to_string());
        res.set("OriginalFilename", &format!("{}-freecam", name).to_string());

        res.compile().unwrap();
    }
}
