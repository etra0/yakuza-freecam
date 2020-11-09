use common::common::{get_version, Camera, Injection};
use memory_rs::process::process_wrapper::Process;
use std::f32;
use std::io::Error;
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};
use winapi::shared::windef::POINT;
use winapi::um::winuser;
use winapi::um::winuser::{GetAsyncKeyState, GetCursorPos, SetCursorPos};

const INITIAL_POS: i32 = 500;
static mut ORIGINAL_VAL_UI: [u32; 5] = [0; 5];

extern "C" {
    static get_camera_data: u8;
    static get_camera_data_end: u8;

    static get_pause_value: u8;
    static get_pause_value_end: u8;

    static get_controller_input: u8;
    static get_controller_input_end: u8;
}


fn detect_activation_by_controller(value: u64) -> bool {
    let result = value & 0x11;
    result == 0x11
}

fn trigger_pause(process: &Process, addr: usize) {
    if addr == 0x0 {
        return;
    }
    process.write_value::<u8>(addr, 0x1, true);
    thread::sleep(Duration::from_millis(100));
    process.write_value::<u8>(addr, 0x0, true);
}

fn remove_ui(process: &Process, activate: bool) {
    let offsets: Vec<usize> = vec![0x291D1DC, 0x291D1D0, 0x291D1EC, 0x291D1E8, 0x291D1E4];

    unsafe {
    if ORIGINAL_VAL_UI[0] == 0 {
        for (i, offset) in offsets.iter().enumerate() {
            ORIGINAL_VAL_UI[i] = process.read_value::<u32>(*offset, false);
        }
    }

    for (i, offset) in offsets.iter().enumerate() {
        if activate {
            process.write_value::<i32>(*offset, -1, false);
        } else {
            process.write_value::<u32>(*offset, ORIGINAL_VAL_UI[i], false);
        }
    }

    }
}

pub fn main() -> Result<(), Error> {
    let mut mouse_pos: POINT = POINT::default();

    // latest mouse positions
    let mut latest_x = 0;
    let mut latest_y = 0;

    println!("Yakuza Kiwami 2 Freecam v{} by @etra0", get_version());
    println!(
        "
    INSTRUCTIONS:

    PAUSE/L2 + X - Activate/Deactivate Free Camera
    DEL - Deattach Mouse

    WASD/Left Stick - Move in the direction you're pointing
    Mouse/Right Stick - Point
    CTRL, SPACE/TRIANGLE, X - Move UP or DOWN

    PG UP, PG DOWN/DPAD UP, DPAD DOWN - Increase/Decrease speed multiplier
    DPAD LEFT, DPAD RIGHT - Increase/Decrease Right Stick Sensitivity
    F1, F2/L2, R2 - Increase/Decrease FOV respectively
    Q, E/L1, R1 - Rotate the camera

    WARNING: Don't forget to deactivate the freecam before skipping a cutscene
    (it may cause a game freeze)

    WARNING: Once you deattach the camera (PAUSE), your mouse will be set in a fixed
    position, so in order to attach/deattach the mouse to the camera, you can
    press DEL
    "
    );

    println!("Waiting for the game to start");
    let yakuza = loop {
        if let Ok(p) = Process::new("YakuzaKiwami2.exe") {
            break Rc::new(p);
        };

        thread::sleep(Duration::from_secs(5));
    };
    println!("Game hooked");

    let entry_point: usize = 0x1F0222B;

    let p_shellcode = unsafe {
        yakuza.inject_shellcode(
            entry_point,
            9,
            &get_camera_data as *const u8,
            &get_camera_data_end as *const u8,
        )
    };

    let p_controller = unsafe {
        yakuza.inject_shellcode(
            0x1B98487,
            8,
            &get_controller_input as *const u8,
            &get_controller_input_end as *const u8,
        )
    };

    let pause_value_ep: usize = 0xDF5E1B;
    let pause_value = unsafe {
        yakuza.inject_shellcode(
            pause_value_ep,
            7,
            &get_pause_value as *const u8,
            &get_pause_value_end as *const u8,
        )
    };

    let mut cam = Camera::new(yakuza.clone(), p_shellcode);

    // function that changes the focal length of the cinematics, when
    // active, nop this
    cam.injections.push(Injection {
        entry_point: 0xB78D87,
        f_orig: vec![0x89, 0x86, 0xB8, 0x00, 0x00, 0x00],
        f_rep: vec![0x90; 6],
    });

    // nop the setcursorpos inside the game
    cam.injections.push(Injection {
        entry_point: 0x1BA285B,
        f_orig: vec![0xFF, 0x15, 0x47, 0x52, 0x4A, 0x00],
        f_rep: vec![0x90; 6],
    });

    // WIP: Pause the cinematics of the world.
    cam.injections.push(Injection {
        entry_point: 0xDF6F86,
        f_orig: vec![0x0F, 0x84, 0x5E, 0x02, 0x00, 0x00],
        f_rep: vec![0xE9, 0x5F, 0x02, 0x00, 0x00, 0x90],
    });

    // Hide UI stuff
    cam.injections.push(Injection {
        entry_point: 0x8B2E8C,
        f_orig: vec![0x41, 0x0F, 0x29, 0x9E, 0x70, 0x01, 0x00, 0x00],
        f_rep: vec![0x45, 0x0F, 0x29, 0x8E, 0x70, 0x01, 0x00, 0x00],
    });

    // flashy health bar
    cam.injections.push(Injection {
        entry_point: 0x1B71453,
        f_orig: vec![0xC6, 0x04, 0x0B, 0x01],
        f_rep: vec![0xC6, 0x04, 0x0B, 0x00],
    });

    // Nop UI coords writers
    cam.injections.push(Injection {
        entry_point: 0x1F0CB72,
        f_orig: vec![0x89, 0x05, 0x64, 0x06, 0xA1, 0x00, 0x89, 0x0D, 0x52, 0x06, 0xA1, 0x00, 0x89, 0x05, 0x68, 0x06, 0xA1, 0x00, 0x89, 0x0D, 0x5E, 0x06, 0xA1, 0x00],
        f_rep: vec![0x90; 24]
    });

    let mut active = false;
    let mut capture_mouse = false;

    let mut restart_mouse = false;

    loop {
        if capture_mouse & restart_mouse {
            unsafe { SetCursorPos(INITIAL_POS, INITIAL_POS) };
            restart_mouse = !restart_mouse;
            latest_x = INITIAL_POS;
            latest_y = INITIAL_POS;
            continue;
        }

        let start = Instant::now();

        // poll rate
        thread::sleep(Duration::from_millis(10));
        unsafe { GetCursorPos(&mut mouse_pos) };
        let duration = start.elapsed().as_millis() as f32;

        let speed_x = ((mouse_pos.x - latest_x) as f32) / duration;
        let speed_y = ((mouse_pos.y - latest_y) as f32) / duration;

        let c_v_a = yakuza.read_value::<usize>(pause_value + 0x200, true);
        let controller_structure_p: usize = yakuza.read_value(p_controller + 0x200, true);
        let controller_state = match controller_structure_p {
            0 => 0,
            _ => yakuza.read_value::<u64>(controller_structure_p, true),
        };

        if active && capture_mouse {
            cam.update_position(speed_x, speed_y);
            unsafe { cam.handle_keyboard_input() };
        }

        if active && (controller_structure_p != 0x0) {
            let [pos_x, pos_y, pitch, yaw] =
                yakuza.read_value::<[f32; 4]>(controller_structure_p + 0x10, true);

            // L2 & R2 check
            match controller_state & 0x30 {
                0x20 => cam.update_fov(0.01),
                0x10 => cam.update_fov(-0.01),
                _ => (),
            };

            let dp_up = match controller_state & 0x9 {
                0x01 => -2f32,
                0x08 => 2f32,
                _ => 0f32,
            };

            let speed: i8 = match controller_state & 0x3000 {
                0x1000 => 1,
                0x2000 => -1,
                _ => 0,
            };

            let dir_speed: i8 = match controller_state & 0xC000 {
                0x4000 => -1,
                0x8000 => 1,
                _ => 0,
            };

            let rotation: i8 = match controller_state & 0xC0 {
                0x40 => 1,
                0x80 => -1,
                0xC0 => 2,
                _ => 0,
            };

            cam.update_values(-pos_y, -pos_x, dp_up, speed, dir_speed, rotation);
            cam.update_position(pitch, yaw);
        }

        latest_x = mouse_pos.x;
        latest_y = mouse_pos.y;

        // to scroll infinitely
        restart_mouse = !restart_mouse;
        unsafe {
            if detect_activation_by_controller(controller_state)
                || ((GetAsyncKeyState(winuser::VK_PAUSE) as u32 & 0x8000) != 0)
            {
                active = !active;
                if !detect_activation_by_controller(controller_state) {
                    capture_mouse = active;
                }

                let c_status = if active { "Deattached" } else { "Attached" };
                println!("status of camera: {}", c_status);

                if active {
                    cam.deattach();
                    remove_ui(&yakuza, true);
                } else {
                    cam.attach();
                    remove_ui(&yakuza, false);
                }

                trigger_pause(&yakuza, c_v_a);
                thread::sleep(Duration::from_millis(500));
            }

            if (GetAsyncKeyState(winuser::VK_HOME) as u32 & 0x8000) != 0 {
                active = !active;
                capture_mouse = active;

                let c_status = if active { "Deattached" } else { "Attached" };
                println!("status of camera: {}", c_status);

                if active {
                    cam.deattach();
                    remove_ui(&yakuza, true);
                } else {
                    remove_ui(&yakuza, false);
                    cam.attach();
                }

                thread::sleep(Duration::from_millis(500));
            }

            if (GetAsyncKeyState(winuser::VK_END) as u32 & 0x8000) != 0 {
                active = !active;
                capture_mouse = active;

                let c_status = if active { "Deattached" } else { "Attached" };
                println!("status of camera: {}", c_status);

                if active {
                    remove_ui(&yakuza, true);
                    cam.deattach();
                } else {
                    remove_ui(&yakuza, false);
                    cam.attach();
                }
                thread::sleep(Duration::from_millis(500));
            }

            if active & (GetAsyncKeyState(winuser::VK_DELETE) as u32 & 0x8000 != 0) {
                capture_mouse = !capture_mouse;
                let c_status = if !capture_mouse {
                    "Deattached"
                } else {
                    "Attached"
                };
                println!("status of mouse: {}", c_status);
                thread::sleep(Duration::from_millis(500));
            }
        }
    }
}
