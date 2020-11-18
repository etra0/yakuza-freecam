pub mod globals;

use common::external::{Camera, error_message};
use common::internal::Injection;
use memory_rs::{try_winapi, generate_aob_pattern};
use memory_rs::internal::memory::{hook_function, scan_aob, write_aob};
use memory_rs::internal::process_info::ProcessInfo;
use nalgebra_glm as glm;
use std::fs::OpenOptions;
use std::io::prelude::*;
use winapi::shared::minwindef::LPVOID;
use winapi::um::xinput;
use winapi;

use log::{error, info};
use slog;
use slog::o;
use slog::Drain;
use slog_scope;
use slog_stdlog;
use slog_term;
use crate::globals::*;

// TODO: remove this
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

extern "C" {
    static get_camera_data: u8;
    static get_timestop: u8;
}

#[repr(C)]
#[derive(Debug)]
struct GameCamera {
    pos: [f32; 4],
    focus: [f32; 4],
    rot: [f32; 4],
}

pub unsafe extern "system" fn wrapper(lib: LPVOID) -> u32 {
    // Logging initialization
    {
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
                unsafe {
                    use winapi::um::wincon::FreeConsole;
                    (FreeConsole());
                }
                error_message(&msg);
            }
        };

        info!("Exiting");
    }

    winapi::um::libloaderapi::FreeLibraryAndExitThread(
        lib as winapi::shared::minwindef::HMODULE,
        0,
    );

    0
}

macro_rules! scan_aob_process {
    ($proc:expr, $fail:expr, [$($tt:tt),*]) => {{
        let (size, func) = generate_aob_pattern![
            $($tt),*
        ];
        scan_aob($proc.addr, $proc.size, func, size)?.ok_or($fail)?
    }}
}


fn patch(lib: LPVOID) -> Result<()> {
    unsafe {
        use winapi::um::consoleapi::AllocConsole;
        try_winapi!(AllocConsole());
    }

    let proc_inf = ProcessInfo::new("YakuzaLikeADragon.exe")?;
    let mut injections: Vec<Injection> = Vec::new();

    let camera_func: usize = scan_aob_process!(proc_inf, "Couldn't find camera",
        [0x90, 0xC5, 0xF8, 0x10, 0x07, 0xC5, 0xF8, 0x11, 0x86, 0x80, 0x00,
        0x00, 0x00]) - 0x33;
    info!("Camera_func found: {:x}", camera_func);

    let timestop_func: usize = scan_aob_process!(proc_inf, "Couldn't find timestop",
        [0xC5, 0x7A, 0x11, 0x05, 0xC2, 0x32, 0xF8, 0x01, 0xC5, 0xFA, 0x11,
        0x35, 0xBE, 0x32, 0xF8, 0x01]);

    let original_bytes = vec![
        0x40, 0x57, 0x48, 0x83, 0xEC, 0x40, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE,
        0xFF, 0xFF, 0xFF
    ];

    let timestop_bytes = vec![
        0xC5, 0x7A, 0x11, 0x05, 0xC2, 0x32, 0xF8, 0x01, 0xC5, 0xFA, 0x11, 0x35,
        0xBE, 0x32, 0xF8, 0x01
    ];

    macro_rules! auto_cast {
        ($val:expr) => {
            &$val as *const u8 as usize
        };
    };

    info!("Injecting camera_func");
    unsafe {
        hook_function(camera_func, auto_cast!(get_camera_data),
            Some(&mut _get_camera_data), 15)?;

        hook_function(timestop_func, auto_cast!(get_timestop),
            Some(&mut _get_timestop), 16)?;
    }

    info!("Injection completed succesfully");
    let mut xinput_state: xinput::XINPUT_STATE = unsafe { std::mem::zeroed() };

    let mut active = true;
    info!("Starting main loop");
    let mut current_speed = 1_f32;
    let mut first_gc = 0_usize;

    loop {
        unsafe {
            xinput::XInputGetState(0, &mut xinput_state);
        }
        let gp = xinput_state.Gamepad;

        if (gp.wButtons & (0x40 | 0x80)) == (0x40 | 0x80) {
            active = !active;
            println!("Camera is {}", active);
            unsafe {
                _camera_active = active as u8;
            }
            if !active {
                unsafe {
                    _camera_struct = 0;
                    current_speed = 1.;
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        if (gp.wButtons & 0x100) == 0x100 {
            current_speed -= 0.01;
            if current_speed < 1e-2 {
                current_speed = 1e-2;
            }
        }

        if (gp.wButtons & 0x200) == 0x200 {
            current_speed += 0.01;
        }

        if (gp.wButtons & (0x1000 | 0x4000)) == (0x1000 | 0x4000) {
            info!("Exiting main loop");
            unsafe {
                _camera_active = 0;
            }
            break
        }

        unsafe {
            if _camera_struct == 0x0 {
                println!("Camera struct is zero");
                continue;
            }
        }

        if !active {
            continue;
        }

        let gc = unsafe { (_camera_struct + 0x80) as *mut GameCamera };
        unsafe {
        if first_gc != (_camera_struct + 0x80) {
            first_gc = _camera_struct + 0x80;
            println!("{:x}", first_gc);
        }
        }
        let rot = [0., 1., 0.];
        unsafe {
            std::ptr::copy_nonoverlapping(rot.as_ptr(), (*gc).rot.as_mut_ptr(),
                3);
        }


        unsafe {
        _engine_speed = current_speed;
        }
        let r_cam_x = unsafe { (*gc).focus[0] - (*gc).pos[0] };
        let r_cam_y = unsafe { (*gc).focus[1] - (*gc).pos[1] };
        let r_cam_z = unsafe { (*gc).focus[2] - (*gc).pos[2] };

        let p_speed_x = -(gp.sThumbLX as f32) / ((i16::MAX as f32)*1e2);
        let p_speed_y = (gp.sThumbLY as f32)  / ((i16::MAX as f32)*1e2);
        let speed_x = (gp.sThumbRX as f32)    / ((i16::MAX as f32)*1e2);
        let speed_y = -(gp.sThumbRY as f32)   / ((i16::MAX as f32)*1e2);

        let (r_cam_x, r_cam_z, r_cam_y) =
            Camera::calc_new_focus_point(r_cam_x, r_cam_z, r_cam_y,
                speed_x, speed_y);

        unsafe {
            (*gc).pos[0] = (*gc).pos[0] + r_cam_x*p_speed_y +
                p_speed_x*r_cam_z;
            (*gc).pos[1] = (*gc).pos[1] + r_cam_y*p_speed_y;

            (*gc).pos[2] = (*gc).pos[2] + r_cam_z*p_speed_y -
                p_speed_x*r_cam_x;

            (*gc).focus[0] = (*gc).pos[0] + r_cam_x;
            (*gc).focus[1] = (*gc).pos[1] + r_cam_y;
            (*gc).focus[2] = (*gc).pos[2] + r_cam_z;

            println!("{:?} ; {:x}, {}", (*gc), gp.wButtons, current_speed);
        }

        std::thread::sleep(std::time::Duration::from_millis(10));


    }

    std::io::stdout().flush()?;

    info!("Writing original bytes");
    unsafe {
        write_aob(camera_func, &original_bytes)?;
        write_aob(timestop_func, &timestop_bytes)?;
    }

    info!("Freeing console");
    unsafe {
        use winapi::um::wincon::FreeConsole;
        try_winapi!(FreeConsole());
    }

    info!("Exiting library");

    Ok(())
}

memory_rs::main_dll!(wrapper);
