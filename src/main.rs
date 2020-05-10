#![feature(naked_functions)]
#![feature(llvm_asm)]
use sysinfo::{ProcessExt, RefreshKind, SystemExt};

#[cfg(feature = "kiwami2")]
mod kiwami2;

#[cfg(not(feature = "kiwami2"))]
mod zero;

fn should_wait_before_exiting() -> bool {
    let current_pid = match sysinfo::get_current_pid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let system = sysinfo::System::new_with_specifics(RefreshKind::new().with_processes());

    let current_process = match system.get_process(current_pid) {
        Some(process) => process,
        None => return false,
    };

    let parent_pid = match current_process.parent() {
        Some(ppid) => ppid,
        None => return false,
    };

    let parent_process = match system.get_process(parent_pid) {
        Some(parent) => parent,
        None => return false,
    };

    let parent_exe = match parent_process.exe().canonicalize() {
        Ok(exe) => exe,
        Err(_) => return false,
    };

    if parent_exe.ends_with("cargo.exe") {
    	return false
    }

    let system_directory = unsafe {
        let mut buffer: [u16; 255] = std::mem::zeroed();
        let read_len =
            winapi::um::sysinfoapi::GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32);
        String::from_utf16_lossy(&buffer[0..(read_len as usize)])
    };

    let cmd_exe = match std::path::Path::new(&system_directory)
        .join("cmd.exe")
        .canonicalize()
    {
        Ok(cmd) => cmd,
        Err(_) => return false,
    };

    return parent_exe != cmd_exe;
}

fn main() {
    #[cfg(feature = "kiwami2")]
    let result = kiwami2::main();

    #[cfg(not(feature = "kiwami2"))]
    let result = zero::main();

    // Slightly nicer way to print exit codes
    std::process::exit(match result {
        Ok(_) => 0,
        Err(error) => {
            eprintln!("Error: {}", error);

            // Be extra super nice and wait if we're not run from command-line
            if should_wait_before_exiting() {
                dont_disappear::any_key_to_continue::default();
            }

            1
        }
    })
}
