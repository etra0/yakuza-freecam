extern crate winres;

fn main() {
    let mut res = winres::WindowsResource::new();

    // res.set("InternalName", "judgment-freecam");
    res.set_icon("../../assets/judgment.ico").set("InternalName", "judgment-freecam");

    res.compile().unwrap();
}
