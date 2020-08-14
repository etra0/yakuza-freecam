use memory_rs::process::process_wrapper::Process;
use winapi::um::winuser;
use nalgebra_glm as glm;

const CARGO_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
const GIT_VERSION: Option<&'static str> = option_env!("GIT_VERSION");

pub fn get_version() -> String {
    let cargo = CARGO_VERSION.unwrap_or("Unknown");
    let git = GIT_VERSION.unwrap_or("Unknown");

    return format!("{}.{}", cargo, git);
}

pub struct Injection {
    pub entry_point: usize,
    // Original bytes
    pub f_orig: Vec<u8>,
    // Representation to be replaced in case kind == CHANGE
    pub f_rep: Vec<u8>,
}

pub struct Camera<'a> {
    process: &'a Process,
    // Camera position
    p_cam_x: f32,
    p_cam_y: f32,
    p_cam_z: f32,

    // Camera focus position
    f_cam_x: f32,
    f_cam_y: f32,
    f_cam_z: f32,

    dp_forward: f32,
    dp_sides: f32,
    dp_up: f32,

    speed_scale: f32,
    dir_speed_scale: f32,
    rotation: f32,

    // base address for the data
    data_base_addr: usize,

    fov: f32,

    pub injections: Vec<Injection>,
}

impl Camera<'_> {
    pub fn new<'a>(process: &'a Process, data_base_addr: usize) -> Camera {
        Camera {
            process,
            p_cam_x: 0f32,
            p_cam_y: 0f32,
            p_cam_z: 0f32,
            f_cam_x: 0f32,
            f_cam_y: 0f32,
            f_cam_z: 0f32,
            dp_forward: 0f32,
            dp_sides: 0f32,
            dp_up: 0f32,
            speed_scale: 0.01,
            dir_speed_scale: 0.05,
            rotation: 0f32,
            data_base_addr,
            fov: 0f32,
            injections: vec![],
        }
    }

    fn calc_new_focus_point(
        cam_x: f32,
        cam_z: f32,
        cam_y: f32,
        speed_x: f32,
        speed_y: f32,
    ) -> (f32, f32, f32) {
        // use spherical coordinates to add speed
        let theta = cam_z.atan2(cam_x) + speed_x;

        let phi = (cam_x.powi(2) + cam_z.powi(2)).sqrt().atan2(cam_y) + speed_y;

        let r = (cam_x.powi(2) + cam_y.powi(2) + cam_z.powi(2)).sqrt();

        let r_cam_x = r * theta.cos() * phi.sin();
        let r_cam_z = r * theta.sin() * phi.sin();
        let r_cam_y = r * phi.cos();

        (r_cam_x, r_cam_z, r_cam_y)
    }

    pub fn calculate_rotation(focus: glm::Vec3, pos: glm::Vec3, rotation: f32) -> [f32; 3] {
        let up = glm::vec3(0., 1., 0.);

        let m_look_at = glm::look_at(&focus, &pos, &up);
        let direction = {
            let row = m_look_at.row(2);
            glm::vec3(row[0], row[1], row[2])
        };
        // let axis = glm::vec3(0., 0., 1.);
        let m_new = glm::rotate_normalized_axis(&m_look_at, rotation, &direction);

        let result = m_new.row(1);
        
        return [result[0], result[1], result[2]];
    }

    pub fn update_fov(&mut self, delta: f32) {
        if (delta < 0f32) & (self.fov < 0.1) {
            return;
        }
        if (delta > 0f32) & (self.fov > 3.13) {
            return;
        }
        self.fov += delta;
        self.process
            .write_value::<f32>(self.data_base_addr + 0x260, self.fov, true);
    }

    pub unsafe fn handle_keyboard_input(&mut self) {
        let mut dp_forward = 0f32;
        let mut dp_sides = 0f32;
        let mut dp_up = 0f32;
        let mut speed_scale: i8 = 0;
        let mut dir_speed: i8 = 0;
        let mut rotation: i8 = 0;

        if (winuser::GetAsyncKeyState(0x57) as u32 & 0x8000) != 0 {
            dp_forward = 1.;
        }
        if (winuser::GetAsyncKeyState(0x53) as u32 & 0x8000) != 0 {
            dp_forward = -1.;
        }

        if (winuser::GetAsyncKeyState(0x41) as u32 & 0x8000) != 0 {
            dp_sides = 1.;
        }
        if (winuser::GetAsyncKeyState(0x44) as u32 & 0x8000) != 0 {
            dp_sides = -1.;
        }

        if (winuser::GetAsyncKeyState(winuser::VK_SPACE) as u32 & 0x8000) != 0 {
            dp_up = 1.;
        }
        if (winuser::GetAsyncKeyState(winuser::VK_CONTROL) as u32 & 0x8000) != 0 {
            dp_up = -1.;
        }

        if (winuser::GetAsyncKeyState(winuser::VK_F1) as u32 & 0x8000) != 0 {
            self.update_fov(0.01);
        }
        if (winuser::GetAsyncKeyState(winuser::VK_F2) as u32 & 0x8000) != 0 {
            self.update_fov(-0.01)
        }

        if (winuser::GetAsyncKeyState(winuser::VK_PRIOR) as u32 & 0x8000) != 0 {
            speed_scale = 1;
        }

        if (winuser::GetAsyncKeyState(winuser::VK_NEXT) as u32 & 0x8000) != 0 {
            speed_scale = -1;
        }

        if (winuser::GetAsyncKeyState(winuser::VK_F3) as u32 & 0x8000) != 0 {
            dir_speed = -1;
        }

        if (winuser::GetAsyncKeyState(winuser::VK_F4) as u32 & 0x8000) != 0 {
            dir_speed = 1;
        }

        if (winuser::GetAsyncKeyState(0x51) as u32 & 0x8000) != 0 {
            rotation = -1;
        }

        if (winuser::GetAsyncKeyState(0x45) as u32 & 0x8000) != 0 {
            rotation = 1;
        }

        self.update_values(dp_forward, dp_sides, dp_up, speed_scale, dir_speed, rotation);
    }

    pub fn update_values(&mut self, dp_forward: f32, dp_sides: f32, dp_up: f32,
        speed_scale: i8, dir_speed_scale: i8, rotation: i8) {
        self.dp_forward = dp_forward * self.speed_scale;
        self.dp_sides = dp_sides * self.speed_scale;
        self.dp_up = dp_up * self.speed_scale;

        match speed_scale {
            1 => {
                self.speed_scale += 5e-5;
            },
            -1 => {
                if self.speed_scale > 1e-5 {
                    self.speed_scale -= 5e-5;
                } else {
                    println!("Speed couldn't decrease");
                }
            },
            _ => ()
        };

        match dir_speed_scale {
            1 => {
                self.dir_speed_scale += 5e-5;
            },
            -1 => {
                if self.dir_speed_scale > 1e-5 {
                    self.dir_speed_scale -= 5e-5;
                } else {
                    println!("Speed couldn't decrease");
                }
            },
            _ => ()
        };

        match rotation {
            1 => { self.rotation -= 0.01; },
            -1 => { self.rotation += 0.01; },
            2 => { self.rotation = 0.; },
            _ => ()
        };
    }

    pub fn update_position(&mut self, yaw: f32, pitch: f32) {
        self.f_cam_x = self
            .process
            .read_value::<f32>(self.data_base_addr + 0x200, true);
        self.f_cam_y = self
            .process
            .read_value::<f32>(self.data_base_addr + 0x204, true);
        self.f_cam_z = self
            .process
            .read_value::<f32>(self.data_base_addr + 0x208, true);

        self.p_cam_x = self
            .process
            .read_value::<f32>(self.data_base_addr + 0x220, true);
        self.p_cam_y = self
            .process
            .read_value::<f32>(self.data_base_addr + 0x224, true);
        self.p_cam_z = self
            .process
            .read_value::<f32>(self.data_base_addr + 0x228, true);

        self.fov = self
            .process
            .read_value::<f32>(self.data_base_addr + 0x260, true);

        let [up_x, up_y, up_z] = self
            .process
            .read_value::<[f32; 3]>(self.data_base_addr + 0x240, true);

        let up_v = glm::vec3(up_x, up_y, up_z);

        let r_cam_x = self.f_cam_x - self.p_cam_x;
        let r_cam_y = self.f_cam_y - self.p_cam_y;
        let r_cam_z = self.f_cam_z - self.p_cam_z;

        let pitch = pitch * self.dir_speed_scale;
        let yaw = yaw * self.dir_speed_scale;


        let (r_cam_x, r_cam_z, r_cam_y) =
            Camera::calc_new_focus_point(r_cam_x, r_cam_z, r_cam_y, yaw, pitch);

        let pf = glm::vec3(self.f_cam_x, self.f_cam_y, self.f_cam_z);
        let pp = glm::vec3(self.p_cam_x, self.p_cam_y, self.p_cam_z);

        let up_new = Camera::calculate_rotation(pf, pp, self.rotation);
        let up_v = up_new;

        self.f_cam_x = self.p_cam_x + r_cam_x + self.dp_forward * r_cam_x + self.dp_sides * r_cam_z;
        self.f_cam_z = self.p_cam_z + r_cam_z + self.dp_forward * r_cam_z - self.dp_sides * r_cam_x;
        self.f_cam_y = self.p_cam_y + r_cam_y + self.dp_forward * r_cam_y + self.dp_up;

        self.p_cam_x = self.p_cam_x + self.dp_forward * r_cam_x + self.dp_sides * r_cam_z;
        self.p_cam_z = self.p_cam_z + self.dp_forward * r_cam_z - self.dp_sides * r_cam_x;
        self.p_cam_y = self.p_cam_y + self.dp_forward * r_cam_y + self.dp_up;

        // flush movement
        self.dp_forward = 0f32;
        self.dp_up = 0f32;
        self.dp_sides = 0f32;

        self.process
            .write_value::<f32>(self.data_base_addr + 0x200, self.f_cam_x, true);
        self.process
            .write_value::<f32>(self.data_base_addr + 0x204, self.f_cam_y, true);
        self.process
            .write_value::<f32>(self.data_base_addr + 0x208, self.f_cam_z, true);

        self.process
            .write_value::<f32>(self.data_base_addr + 0x220, self.p_cam_x, true);
        self.process
            .write_value::<f32>(self.data_base_addr + 0x224, self.p_cam_y, true);
        self.process
            .write_value::<f32>(self.data_base_addr + 0x228, self.p_cam_z, true);

        self.process
            .write_value::<[f32; 3]>(self.data_base_addr + 0x240, up_v, true);
    }

    pub fn deattach(&self) {
        self.process
            .write_value::<u32>(self.data_base_addr + 0x1F0, 1, true);
        for injection in &self.injections {
            self.process
                .write_aob(injection.entry_point, &injection.f_rep, false);
        }
    }

    pub fn attach(&self) {
        self.process
            .write_value::<u32>(self.data_base_addr + 0x1F0, 0, true);
        for injection in &self.injections {
            self.process
                .write_aob(injection.entry_point, &injection.f_orig, false);
        }
    }
}
