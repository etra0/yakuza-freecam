use winapi::um::xinput;
use memory_rs::try_winapi;
use anyhow::Result;
use log::info;

static mut WARNED_ALREADY: bool = false;

const DEADZONE: i16 = 2000;

pub struct Input {
    pub engine_speed: f32,
    // Deltas with X and Y
    pub delta_pos: (f32, f32),
    pub delta_focus: (f32, f32),

    pub delta_altitude: f32,

    pub change_active: bool,

    pub fov: f32,
    #[cfg(debug_assertions)]
    pub deattach: bool
}

impl Input {
    pub fn new() -> Input {
        let mut input: Input = unsafe { std::mem::zeroed() };
        input.reset();

        input.fov = 0.92;
        input.engine_speed = 0.;

        input
    }

    pub fn reset(&mut self) {
        self.delta_pos = (0., 0.);
        self.delta_focus = (0., 0.);
        self.delta_altitude = 0.;
        self.change_active = false;

        #[cfg(debug_assertions)]
        {
        self.deattach = false;
        }
    }

    pub fn sanitize(&mut self) {
        if self.fov < 1e-3 {
            self.fov = 0.01;
        }
        if self.fov > 3.12 {
            self.fov = 3.12;
        }

        if self.engine_speed < 1e-4 {
            self.engine_speed = 1e-4;
        }
    }
}

// Asume safety of XInputGameState
fn xinput_get_state(xinput_state: &mut xinput::XINPUT_STATE) -> Result<()> {
    use xinput::XInputGetState;
    let xinput_wrapper = |xs: &mut xinput::XINPUT_STATE| -> u32 {
        let res = unsafe { XInputGetState(0, xs) };
        if res == 0 {
            return 1;
        }
        return 0;
    };

    try_winapi!(
        xinput_wrapper(xinput_state)
    );

    Ok(())
}

pub fn handle_controller(input: &mut Input) {
    let mut xs: xinput::XINPUT_STATE = unsafe { std::mem::zeroed() };
    if xinput_get_state(&mut xs).is_err() {
        unsafe {
            if !WARNED_ALREADY {
                info!("Controller was not detected on first try");
                WARNED_ALREADY = true;
            }
        }
        return;
    }

    unsafe {
        xinput::XInputEnable(1);
    }

    let gp = xs.Gamepad;

    // check camera activation
    if (gp.wButtons & (0x200 | 0x80)) == (0x200 | 0x80) {
        input.change_active = true;
    }

    // modify speed
    if (gp.wButtons & 0x4) == 0x4 {
        input.engine_speed -= 0.01;
    }
    if (gp.wButtons & 0x8) == 0x8 {
        input.engine_speed += 0.01;
    }

    if gp.bLeftTrigger > 150 {
        input.fov -= 0.01;
    }

    if gp.bRightTrigger > 150 {
        input.fov += 0.01;
    }

    macro_rules! dead_zone {
        ($val:expr) => {
            if ($val < DEADZONE) && ($val > -DEADZONE) {
                0
            } else {
                $val
            }
        }
    }

    input.delta_pos.0 = -(dead_zone!(gp.sThumbLX) as f32) / ((i16::MAX as f32)*1e2);
    input.delta_pos.1 = (dead_zone!(gp.sThumbLY) as f32)  / ((i16::MAX as f32)*1e2);

    input.delta_focus.0 = (dead_zone!(gp.sThumbRX) as f32)  / ((i16::MAX as f32)*1e2);
    input.delta_focus.1 = -(dead_zone!(gp.sThumbRY) as f32) / ((i16::MAX as f32)*1e2);

    #[cfg(debug_assertions)]
    if (gp.wButtons & (0x1000 | 0x4000)) == (0x1000 | 0x4000) {
        input.deattach = true;
    }
}
