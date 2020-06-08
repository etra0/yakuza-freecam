extern crate winres;
use std::env;
use std::path::Path;

fn main() {

    let games = vec!["yakuza0", "kiwami", "kiwami2"];

    for game in games {
        let source_dir = format!("src/bin/{}/interceptor.asm", game);
        let ico = format!("assets/{}.ico", game);
        let interceptor_name = format!("interceptor_{}", game);

        cc::Build::new()
            .file(source_dir.as_str())
            .compile("interceptor");
        println!("cargo:rerun-if-changed={}", source_dir.as_str());

    }

    // res.set_icon(&n_ico);
    // res.set("OriginalFilename", &format!("{}-freecam", name).to_string());

}
