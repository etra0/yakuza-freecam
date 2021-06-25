extern crate winres;

fn main() {
    let mut res = winres::WindowsResource::new();

    // res.set_icon("../../assets/likeadragon.ico")
    //     .set("InternalName", "likeadragon-freecam");

    res.compile().unwrap();
}
