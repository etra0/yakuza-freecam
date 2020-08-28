use common::common::{get_version, Camera, Injection};
use memory_rs::process::process_wrapper::Process;
use std::f32;
use std::io::Error;
use std::thread;
use std::time::{Duration, Instant};
use winapi::shared::windef::POINT;
use winapi::um::winuser;
use winapi::um::winuser::{GetAsyncKeyState, GetCursorPos, SetCursorPos};

const INITIAL_POS: i32 = 500;

extern "C" {
    static get_camera_data: u8;
    static get_camera_data_end: u8;

    static get_controller_input: u8;
    static get_controller_input_end: u8;
}

fn detect_activation_by_controller(value: u64, activation: u64) -> bool {
    let result = value & activation;
    result == activation
}

pub fn main() -> Result<(), Error> {
    let mut mouse_pos: POINT = POINT::default();

    // latest mouse positions
    let mut latest_x = 0;
    let mut latest_y = 0;

    println!("Yakuza 0 Freecam v{} by @etra0", get_version());
    println!(
        "
    INSTRUCTIONS:

    PAUSE/L2 + X - Activate/Deactivate Free Camera
    END/L2 + Square - Pause the cinematic
    DEL - Deattach Mouse

    W, A, S, D/Left Stick - Move the camera
    Mouse/Right Stick - Point the camera
    CTRL, SPACE/TRIANGLE, X - Move UP or DOWN

    PG UP, PG DOWN/DPAD UP, DPAD DOWN - Increase/Decrease speed multiplier
    DPAD LEFT, DPAD RIGHT - Increase/Decrease Right Stick Sensitivity
    F1, F2/L2, R2 - Increase/Decrease FOV respectively
    Q, E/L1, R1 - Rotate the camera

    WARNING: Once you deattach the camera (PAUSE), your mouse will be set in a fixed
    position, so in order to attach/deattach the mouse to the camera, you can
    press DEL

    WARNING: If you're in freeroam and you stop hearing audio, it's probably
    because you have the paused option activated, simply press END to deactivate it.

    "
    );

    println!("Waiting for the game to start");
    let yakuza = loop {
        if let Ok(p) = Process::new("Yakuza0.exe") {
            break p;
        };

        thread::sleep(Duration::from_secs(5));
    };
    println!("Game hooked");

    let entry_point: usize = 0x18FD38;
    let p_shellcode = unsafe {
        yakuza.inject_shellcode(
            entry_point,
            5,
            &get_camera_data as *const u8,
            &get_camera_data_end as *const u8,
        )
    };

    let p_controller = unsafe {
        yakuza.inject_shellcode(
            0xEC1F,
            6,
            &get_controller_input as *const u8,
            &get_controller_input_end as *const u8,
        )
    };

    let mut cam = Camera::new(&yakuza, p_shellcode);

    // function that changes the focal length of the cinematics, when
    // active, nop this

    cam.injections.push(Injection {
        entry_point: 0x187616,
        f_orig: vec![0xF3, 0x0F, 0x11, 0x89, 0xAC, 0x00, 0x00, 0x00],
        f_rep: vec![0x90; 8],
    });

    // WIP: Pause the cinematics of the world.
    let pause_cinematic_f: Vec<u8> = vec![0x41, 0x8A, 0x8E, 0xC9, 0x00, 0x00, 0x00];
    let pause_cinematic_rep: Vec<u8> = vec![0xB1, 0x01, 0x90, 0x90, 0x90, 0x90, 0x90];
    let pause_cinematic_offset = 0xB720DE;
    let mut pause_world = false;

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

        let controller_structure_p: usize = yakuza.read_value(p_controller + 0x200, true);
        let controller_state = match controller_structure_p {
            0 => 0,
            _ => yakuza.read_value::<u64>(controller_structure_p, true),
        };

        if active && capture_mouse {
            cam.update_position(speed_x, speed_y);
            unsafe { cam.handle_keyboard_input() };
        }

        if active && (controller_structure_p != 0) {
            let [pos_x, pos_y, pitch, yaw] =
                yakuza.read_value::<[f32; 4]>(controller_structure_p + 0x10, true);

            // L1 & R1 check
            match controller_state & 0x30 {
                0x20 => cam.update_fov(0.01),
                0x10 => cam.update_fov(-0.01),
                _ => (),
            };

            let speed: i8 = match controller_state & 0x3000 {
                0x1000 => 1,
                0x2000 => -1,
                _ => 0,
            };

            let dp_up = match controller_state & 0x9 {
                0x8 => 2f32,
                0x1 => -2f32,
                _ => 0f32,
            };

            let dir_speed = match controller_state & 0xC000 {
                0x8000 => 1,
                0x4000 => -1,
                _ => 0,
            };

            let rotation: i8 = match controller_state & 0xC0 {
                0x40 => 1,
                0x80 => -1,
                0xC0 => 2,
                _ => 0,
            };

            cam.update_values(-pos_y, -pos_x, dp_up, speed, dir_speed, rotation); //dp_up, speed, dir_speed, rotation);
            cam.update_position(pitch, yaw);
        }

        latest_x = mouse_pos.x;
        latest_y = mouse_pos.y;

        // to scroll infinitely
        restart_mouse = !restart_mouse;
        unsafe {
            if detect_activation_by_controller(controller_state, 0x11)
                || (GetAsyncKeyState(winuser::VK_PAUSE) as u32 & 0x8000) != 0
            {
                active = !active;

                if controller_state & 0x11 != 0x11 {
                    capture_mouse = active;
                }

                let c_status = if active { "Deattached" } else { "Attached" };
                println!("status of camera: {}", c_status);
                if active {
                    cam.deattach();
                } else {
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

            if detect_activation_by_controller(controller_state, 0x14)
                || (GetAsyncKeyState(winuser::VK_END) as u32 & 0x8000) != 0
            {
                pause_world = !pause_world;
                println!("status of pausing: {}", pause_world);
                if pause_world {
                    yakuza.write_aob(pause_cinematic_offset, &pause_cinematic_rep, false);
                } else {
                    yakuza.write_aob(pause_cinematic_offset, &pause_cinematic_f, false);
                }
                thread::sleep(Duration::from_millis(500));
            }
        }
    }
}
