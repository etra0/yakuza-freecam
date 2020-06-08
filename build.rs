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

        let n_asm = format!("src\\asm\\{}.asm", name).to_string();
        let n_ico = format!("assets\\{}.ico", name).to_string();
        let n_out_asm = format!("{}-asm", name);

        cc::Build::new()
            .file(&n_asm)
            .compile(&n_out_asm);
        println!("cargo:rerun-if-changed=src\\asm\\{}.asm", name);

        res.set_icon(&n_ico);
        res.set("OriginalFilename", &format!("{}-freecam", name).to_string());

        res.compile().unwrap();
    }
}
