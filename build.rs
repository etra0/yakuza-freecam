extern crate winres;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        if cfg!(feature = "kiwami2") {
            res.set_icon("kiwami2.ico");
        } else {
            res.set_icon("yakuza0.ico");
        }
        res.compile().unwrap();
    }
}
