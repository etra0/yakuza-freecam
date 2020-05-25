use memory_rs::process::process_wrapper::Process;
use winapi::um::winuser;
use std::thread;
use std::time::{Duration, Instant};

// TODO: Fix this pub stuff
pub struct Injection {
    pub entry_point: usize,
    // Original bytes
    pub f_orig: Vec<u8>,
    // Representation to be replaced in case kind == CHANGE
    pub f_rep: Vec<u8>
}

pub struct Camera {
    // Camera position
    p_cam_x: f32,
    p_cam_y: f32,
    p_cam_z: f32,

    // Camera focus position
    f_cam_x: f32,
    f_cam_y: f32,
    f_cam_z: f32,

    speed_scale: f32,

    // base address for the data
    data_base_addr: usize,

    fov: f32,

    pub injections: Vec<Injection>
}

impl Camera {
    pub fn new(data_base_addr: usize) -> Camera {
        Camera {
            p_cam_x: 0.,
            p_cam_y: 0.,
            p_cam_z: 0.,
            f_cam_x: 0.,
            f_cam_y: 0.,
            f_cam_z: 0.,
            speed_scale: 1.,
            data_base_addr,
            fov: 0.,
            injections: vec![]
        }
    }

    fn calc_new_focus_point(cam_x: f32, cam_z: f32,
        cam_y: f32, speed_x: f32, speed_y: f32) -> (f32, f32, f32) {

        // use spherical coordinates to add speed
        let theta = cam_z.atan2(cam_x) + speed_x;

        let phi = (cam_x.powi(2) + cam_z.powi(2)).sqrt().atan2(cam_y) +
            speed_y;

        let r = (cam_x.powi(2) + cam_y.powi(2) + cam_z.powi(2)).sqrt();

        let r_cam_x = r*theta.cos()*phi.sin();
        let r_cam_z = r*theta.sin()*phi.sin();
        let r_cam_y = r*phi.cos();

        (r_cam_x, r_cam_z, r_cam_y)
    }

    pub fn update_position(&mut self, process: &Process, speed_x: f32, speed_y: f32) {
        self.f_cam_x = process.read_value::<f32>(self.data_base_addr + 0x200);
        self.f_cam_y = process.read_value::<f32>(self.data_base_addr + 0x204);
        self.f_cam_z = process.read_value::<f32>(self.data_base_addr + 0x208);

        self.p_cam_x = process.read_value::<f32>(self.data_base_addr + 0x220);
        self.p_cam_y = process.read_value::<f32>(self.data_base_addr + 0x224);
        self.p_cam_z = process.read_value::<f32>(self.data_base_addr + 0x228);

        self.fov = process.read_value::<f32>(self.data_base_addr + 0x260);

        let r_cam_x = self.f_cam_x - self.p_cam_x;
        let r_cam_y = self.f_cam_y - self.p_cam_y;
        let r_cam_z = self.f_cam_z - self.p_cam_z;

        let mut dp_forward = 0.;
        let mut dp_sides = 0.;
        let mut dp_up = 0.;

        unsafe {
            if (winuser::GetAsyncKeyState(winuser::VK_UP) as u32 & 0x8000) != 0 {
                dp_forward = 0.1*self.speed_scale;
            }
            if (winuser::GetAsyncKeyState(winuser::VK_DOWN) as u32 & 0x8000) != 0 {
                dp_forward = -0.1*self.speed_scale;
            }

            if (winuser::GetAsyncKeyState(winuser::VK_LEFT) as u32 & 0x8000) != 0 {
                dp_sides = 0.1*self.speed_scale;
            }
            if (winuser::GetAsyncKeyState(winuser::VK_RIGHT) as u32 & 0x8000) != 0 {
                dp_sides = -0.1*self.speed_scale;
            }

            if (winuser::GetAsyncKeyState(winuser::VK_SPACE) as u32 & 0x8000) != 0 {
                dp_up = 0.1*self.speed_scale;
            }
            if (winuser::GetAsyncKeyState(winuser::VK_CONTROL) as u32 & 0x8000) != 0 {
                dp_up = -0.1*self.speed_scale;
            }

            if (winuser::GetAsyncKeyState(winuser::VK_F1) as u32 & 0x8000) != 0 {
                self.fov += if self.fov < 3.13 { 0.01 } else { 0. };
            }
            if (winuser::GetAsyncKeyState(winuser::VK_F2) as u32 & 0x8000) != 0 {
                self.fov -= if self.fov > 0.1 { 0.01 } else { 0. };
            }

            if (winuser::GetAsyncKeyState(winuser::VK_PRIOR) as u32 & 0x8000) != 0 {
                self.speed_scale *= 2.;
                println!("Speed increased, {:.2}", self.speed_scale);
                thread::sleep(Duration::from_millis(100));
            }

            if (winuser::GetAsyncKeyState(winuser::VK_NEXT) as u32 & 0x8000) != 0 {
                if self.speed_scale > 1e-5 {
                    self.speed_scale /= 2.;
                    println!("Speed decreased, {:.2}", self.speed_scale);
                } else {
                    println!("Cannot be decreased, {:.2}", self.speed_scale);
                }
                thread::sleep(Duration::from_millis(100));
            }
        }

        let (r_cam_x, r_cam_z, r_cam_y) = Camera::calc_new_focus_point(r_cam_x,
            r_cam_z, r_cam_y, speed_x, speed_y);

        self.f_cam_x = self.p_cam_x + r_cam_x + dp_forward*r_cam_x + dp_sides*r_cam_z;
        self.f_cam_z = self.p_cam_z + r_cam_z + dp_forward*r_cam_z - dp_sides*r_cam_x;
        self.f_cam_y = self.p_cam_y + r_cam_y + dp_forward*r_cam_y + dp_up*r_cam_y;

        self.p_cam_x = self.p_cam_x + dp_forward*r_cam_x + dp_sides*r_cam_z;
        self.p_cam_z = self.p_cam_z + dp_forward*r_cam_z - dp_sides*r_cam_x;
        self.p_cam_y = self.p_cam_y + dp_forward*r_cam_y + dp_up*r_cam_y;

        process.write_value::<f32>(self.data_base_addr + 0x200, self.f_cam_x);
        process.write_value::<f32>(self.data_base_addr + 0x204, self.f_cam_y);
        process.write_value::<f32>(self.data_base_addr + 0x208, self.f_cam_z);

        process.write_value::<f32>(self.data_base_addr + 0x220, self.p_cam_x);
        process.write_value::<f32>(self.data_base_addr + 0x224, self.p_cam_y);
        process.write_value::<f32>(self.data_base_addr + 0x228, self.p_cam_z);

        process.write_value::<f32>(self.data_base_addr + 0x260, self.fov);

        // TODO: Generalizar esto
        process.write_value::<[f32; 3]>(self.data_base_addr+0x240,
            [0., 1., 0.]);
    }

    pub fn deattach(&self, process: &Process) {
        process.write_value::<u32>(self.data_base_addr + 0x1F0, 1);
        for injection in &self.injections {
            process.write_aob(injection.entry_point, &injection.f_rep);
        }
    }

    pub fn attach(&self, process: &Process) {
        process.write_value::<u32>(self.data_base_addr + 0x1F0, 0);
        for injection in &self.injections {
            process.write_aob(injection.entry_point, &injection.f_orig);
        }
    }
}
