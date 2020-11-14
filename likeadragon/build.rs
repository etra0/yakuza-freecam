extern crate winres;

fn main() {
    let mut res = winres::WindowsResource::new();

    res.set_icon("../assets/yakuza0.ico");

    println!("cargo:rerun-if-changed=interceptor.asm");
    cc::Build::new()
        .file("src/interceptor.asm")
        .compile("interceptor");

    res.compile().unwrap();
}
