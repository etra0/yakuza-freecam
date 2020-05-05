use memory_rs::process::process_wrapper::Process;

fn main() {
    let yakuza = Process::new("Yakuza0.exe");

    let ending: Vec<u8> = vec![0x0, 0x55];
    let result = yakuza.read_string_array(0x7FF4DDE067E8, 2, &ending);
    for value in result {
        println!("{}:{}", value.0, value.1);
    }
}
