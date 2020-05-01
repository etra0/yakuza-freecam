extern crate winres;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("yakuza0.ico");
        res.compile().unwrap();
    }
}
