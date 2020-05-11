extern crate winres;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();

        #[cfg(feature = "kiwami2")]
        res.set_icon("assets\\kiwami2.ico");

        #[cfg(feature = "kiwami")]
        res.set_icon("assets\\kiwami.ico");

        #[cfg(all(not(feature = "kiwami2"), not(feature = "kiwami")))]
        res.set_icon("assets\\yakuza0.ico");

        res.compile().unwrap();
    }
}
