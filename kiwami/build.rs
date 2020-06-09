extern crate winres;

fn main() {
    let mut res = winres::WindowsResource::new();

    res.set_icon("../assets/kiwami.ico");

    cc::Build::new()
        .file("src/interceptor.asm")
        .compile("interceptor");
    println!("cargo:rerun-if-changed=interceptor.asm");

    res.compile().unwrap();

}
