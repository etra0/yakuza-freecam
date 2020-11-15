use memory_rs::generate_aob_pattern;
use memory_rs::internal::memory::{hook_function, scan_aob, write_aob};
use memory_rs::internal::process_info::ProcessInfo;
use std::ffi::CString;
use std::fs::OpenOptions;
use winapi;
use winapi::shared::minwindef::LPVOID;
use winapi::um::xinput;
use nalgebra_glm as glm;
use common::external::{Camera, error_message};

use log::{error, info};
use slog;
use slog::o;
use slog::Drain;
use slog_scope;
use slog_stdlog;
use slog_term;

extern "C" {
    static get_camera_data: u8;
    static get_camera_data_end: u8;
}

#[no_mangle]
pub static mut _camera_struct: usize = 0;


pub unsafe extern "system" fn wrapper(lib: LPVOID) -> u32 {
    // Logging initialization
    let log_path = "ylad.log";
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path)
        .unwrap();

    let decorator = slog_term::PlainSyncDecorator::new(file);
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let logger = slog::Logger::root(drain, o!());

    let _guard = slog_scope::set_global_logger(logger);

    slog_stdlog::init().unwrap();

    match patch(lib) {
        Ok(_) => {
            info!("Everything executed perfectly");
        }
        Err(e) => {
            let msg = format!("{}", e);
            error!("Error: {}", msg);
            error_message(&msg);
            // Unload the lib in case the injection failed
            winapi::um::libloaderapi::FreeLibraryAndExitThread(
                lib as winapi::shared::minwindef::HMODULE,
                0,
            );
        }
    }

    0
}

fn patch(lib: LPVOID) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        winapi::um::consoleapi::AllocConsole();
    }

    let proc_inf = ProcessInfo::new("YakuzaLikeADragon.exe")?;

    // for now camera_func will be fixed
    let camera_func: usize = proc_inf.addr + 0x2A3110;
    let original_bytes = vec![
        0x40, 0x57, 0x48, 0x83, 0xEC, 0x40, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE,
        0xFF, 0xFF, 0xFF
    ];

    macro_rules! auto_cast {
        ($val:expr) => {
            &$val as *const u8 as usize
        };
    };

    unsafe {
        hook_function(camera_func, auto_cast!(get_camera_data),
            None, 15)?;
    }

    let mut xinput_state: xinput::XINPUT_STATE = unsafe { std::mem::zeroed() };
    std::thread::sleep(std::time::Duration::from_secs(3));

    loop {
        unsafe {
            if _camera_struct == 0x0 {
                println!("Camera struct is zero");
                continue;
            }
            let pos = std::slice::from_raw_parts_mut((_camera_struct+0x80) as *mut f32, 3);
            let focus = std::slice::from_raw_parts_mut((_camera_struct+0x90) as *mut f32, 3);
            let rot = [0., 1., 0.];
            std::ptr::copy_nonoverlapping(rot.as_ptr(), (_camera_struct + 0xA0) as *mut f32, 3);
            xinput::XInputGetState(0, &mut xinput_state);
            let gp = xinput_state.Gamepad;

            let r_cam_x = focus[0] - pos[0];
            let r_cam_y = focus[1] - pos[1];
            let r_cam_z = focus[2] - pos[2];

            let p_speed_x = -(gp.sThumbLX as f32) / ((i16::MAX as f32)*1000.);
            let p_speed_y = (gp.sThumbLY as f32) / ((i16::MAX as f32)*1000.);
            let speed_x = (gp.sThumbRX as f32) / ((i16::MAX as f32)*10000.);
            let speed_y = (gp.sThumbRY as f32) / ((i16::MAX as f32)*10000.);

            let (r_cam_x, r_cam_z, r_cam_y) =
                Camera::calc_new_focus_point(r_cam_x, r_cam_z, r_cam_y,
                    speed_x, speed_y);

            pos[0] = pos[0] + r_cam_x*p_speed_y + p_speed_x*r_cam_z;
            pos[1] = pos[1] + r_cam_y*p_speed_y;
            pos[2] = pos[2] + r_cam_z*p_speed_y - p_speed_x*r_cam_x;

            focus[0] = pos[0] + r_cam_x;
            focus[1] = pos[1] + r_cam_y;
            focus[2] = pos[2] + r_cam_z;


            println!("pos: {:?}, focus: {:?}", pos, focus);

            if (gp.wButtons & 0x1000 != 0) {
                break
            }
        }
    }

    unsafe {
        write_aob(camera_func, &original_bytes);
    }

    unsafe {
        winapi::um::wincon::FreeConsole();
    }
    unsafe { 
        winapi::um::libloaderapi::FreeLibraryAndExitThread(
            lib as winapi::shared::minwindef::HMODULE,
            0,
        );
    }

    Ok(())
}

memory_rs::main_dll!(wrapper);
