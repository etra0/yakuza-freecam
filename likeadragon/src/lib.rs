use memory_rs::generate_aob_pattern;
use memory_rs::internal::memory::{hook_function, scan_aob, write_aob};
use memory_rs::internal::process_info::ProcessInfo;
use std::ffi::CString;
use std::fs::OpenOptions;
use winapi;
use winapi::shared::minwindef::LPVOID;
use winapi::um::xinput;
use nalgebra_glm as glm;
use common::common::Camera;

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

fn error_message(message: &str) {
    let title = CString::new("Error while patching").unwrap();
    let message = CString::new(message).unwrap();

    unsafe {
        winapi::um::winuser::MessageBoxA(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            0x10,
        );
    }
}

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

#[repr(C)]
#[derive(Debug)]
struct GameCamera {
    right: [f32; 4],
    up: [f32; 4],
    direction: [f32; 4],
    pos: [f32; 4]
}

fn calc_new_pos(pos: &Vec<f32>, focus: &Vec<f32>) {
    let p_camera = unsafe { _camera_struct as *mut GameCamera };

    let pos = glm::vec3(pos[0], pos[1], pos[2]);
    let focus = glm::vec3(focus[0], focus[1], focus[2]);
    let up = glm::vec3(0., 1., 0.);

    let mat = glm::look_at(&pos, &focus, &up);

    macro_rules! vec_to_slice {
        ($mat:expr, $ind:expr) => {{
            let _t: glm::Vec4 = glm::row(&$mat, $ind).into();
            let mut _t: [f32; 4] = _t.into();
            _t[3] = 0.;
            _t
        }}
    }

    let mat0: [f32; 4] = vec_to_slice!(mat, 0);
    let mat1: [f32; 4] = vec_to_slice!(mat, 1);
    let mat2: [f32; 4] = vec_to_slice!(mat, 2);
    let pos: [f32; 4] = [pos[0], pos[1], pos[2], 0.];
    println!("{:?}", mat);

    unsafe {
        std::ptr::copy_nonoverlapping(&mat0, &mut (*p_camera).right, 1);
        std::ptr::copy_nonoverlapping(&mat1, &mut (*p_camera).up, 1);
        std::ptr::copy_nonoverlapping(&mat2, &mut (*p_camera).direction, 1);
        std::ptr::copy_nonoverlapping(&pos, &mut (*p_camera).pos, 1);
    }
}

fn patch(lib: LPVOID) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        winapi::um::consoleapi::AllocConsole();
    }

    let proc_inf = ProcessInfo::new("YakuzaLikeADragon.exe")?;

    // for now camera_func will be fixed
    let camera_func: usize = proc_inf.addr + 0x1F41D1B;
    let original_bytes = vec![0x49, 0x8D, 0x50, 0x40, 0x48, 0x8D, 0x4F, 0x20,
        0xE8, 0xB8, 0x76, 0x03, 0x00];
    // let camera_func = {
    //     let (size, func) = generate_aob_pattern![
    //         0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x60, 0xC5, 0xF8, 0x10, 0x02
    //     ];
    //     scan_aob(proc_inf.addr, proc_inf.size, func, size)?.ok_or("Couldn't find func")?
    // };

    macro_rules! auto_cast {
        ($val:expr) => {
            &$val as *const u8 as usize
        };
    };

    unsafe {
        hook_function(camera_func, auto_cast!(get_camera_data),
            Some(auto_cast!(get_camera_data_end)), 18)?;
    }

    let mut xinput_state: xinput::XINPUT_STATE = unsafe { std::mem::zeroed() };
    let mut pos: Vec<f32> = Vec::new();
    let mut focus: Vec<f32> = Vec::new();
    std::thread::sleep(std::time::Duration::from_secs(3));
    unsafe {
        let a = std::slice::from_raw_parts((_camera_struct + 12*4) as *const
            f32, 4);
        pos.extend_from_slice(a);
        focus.extend_from_slice(a);

        focus[0] += 1.;
        focus[2] += 1.;
    }
    loop {
        unsafe {
            xinput::XInputGetState(0, &mut xinput_state);
            let gp = xinput_state.Gamepad;

            let r_cam_x = focus[0] - pos[0];
            let r_cam_y = focus[1] - pos[1];
            let r_cam_z = focus[2] - pos[2];

            let p_speed_x = (gp.sThumbLY as f32) / ((i16::MAX as f32)*100.);
            let p_speed_y = (gp.sThumbLX as f32) / ((i16::MAX as f32)*100.);
            let speed_x = (gp.sThumbRX as f32) / ((i16::MAX as f32)*10000.);
            let speed_y = (gp.sThumbRY as f32) / ((i16::MAX as f32)*10000.);

            let (r_cam_x, r_cam_z, r_cam_y) =
                Camera::calc_new_focus_point(r_cam_x, r_cam_z, r_cam_y,
                    speed_x, speed_y);

            // println!("r: {} {} {}", r_cam_x, r_cam_y, r_cam_z);

            // x
            pos[0] = pos[0] + r_cam_x*p_speed_y + p_speed_x*r_cam_z;
            // y
            pos[1] = pos[1] + r_cam_y*p_speed_y;
            // z
            pos[2] = pos[2] + r_cam_z*p_speed_y - p_speed_x*r_cam_x;

            focus[0] = pos[0] + r_cam_x;
            focus[1] = pos[1] + r_cam_y;
            focus[2] = pos[2] + r_cam_z;


            calc_new_pos(&pos, &focus);

            // println!("pos: {:?}, focus: {:?}", pos, focus);

            if (gp.wButtons & 0x1000 != 0) {
                break
            }
        }
    }

    // println!("Press a key to exit");
    // {
    //     let mut b = String::new();
    //     let stdin = std::io::stdin();
    //     stdin.read_line(&mut b).unwrap();
    // }

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
