use crate::camera::*;
use crate::utils::*;
use nalgebra_glm as glm;
use std::time::Duration;
use winapi::um::winuser;

#[derive(Debug, Clone)]
pub struct CameraSnapshot {
    pub pos: glm::TVec3<f32>,
    pub focus: glm::TVec3<f32>,
    pub rot: glm::TVec3<f32>,
    pub fov: f32,
}

pub trait Interpolate {
    fn interpolate(&self, gc: &mut GameCamera, duration: Duration, loop_it: bool);
}

impl CameraSnapshot {
    pub fn new(gc: &GameCamera) -> Self {
        let pos: glm::Vec3 = gc.pos.into();
        let focus: glm::Vec3 = gc.focus.into();
        let rot: glm::Vec3 = gc.rot.into();
        let fov = f32::from(gc.fov);

        Self {
            pos,
            focus,
            rot,
            fov,
        }
    }

    pub fn set_inplace(&self, gc: &mut GameCamera) {
        gc.pos = self.pos.into();
        gc.focus = self.focus.into();
        gc.rot = self.rot.into();
        gc.fov = self.fov.into();
    }
}

fn solve_eq(t: f32, p0: glm::Vec3, p1: glm::Vec3, p2: glm::Vec3, p3: glm::Vec3) -> glm::Vec3 {
    let b0 = 0.5 * (-t.powi(3) + 2. * t.powi(2) - t);
    let b1 = 0.5 * (3. * t.powi(3) - 5. * t.powi(2) + 2.);
    let b2 = 0.5 * (-3. * t.powi(3) + 4. * t.powi(2) + t);
    let b3 = 0.5 * (t.powi(3) - t.powi(2));

    p0 * b0 + p1 * b1 + p2 * b2 + p3 * b3
}

impl Interpolate for Vec<CameraSnapshot> {
    fn interpolate(&self, gc: &mut GameCamera, duration: Duration, loop_it: bool) {
        let sleep_duration = Duration::from_millis(10);

        let fraction = sleep_duration.as_secs_f32() / duration.as_secs_f32();

        self[0].set_inplace(gc);

        macro_rules! bounds {
            ($var:expr) => {
                // TODO: Check if this was the issue with the smooth transition
                if $var < 0 {
                    if loop_it {
                        (self.len() - 1) as i32
                    } else {
                        0
                    }
                } else if $var >= (self.len() - 1) as i32 {
                    if loop_it {
                        $var % (self.len()) as i32
                    } else {
                        (self.len() - 1) as i32
                    }
                } else {
                    $var
                }
            };
        }

        let delta_t = if loop_it {
            1. / ((self.len()) as f32)
        } else {
            1. / ((self.len() - 1) as f32)
        };

        'outer: loop {
            let mut t = 0.;
            while t < 1. {
                if check_key_press(winuser::VK_F8) {
                    break 'outer;
                }

                let p: i32 = (t / delta_t) as i32;
                let p0 = bounds!(p - 1) as usize;
                let p1 = bounds!(p) as usize;
                let p2 = bounds!(p + 1) as usize;
                let p3 = bounds!(p + 2) as usize;

                let rt = (t - delta_t * (p as f32)) / delta_t;

                let fov = glm::lerp_scalar(self[p1].fov, self[p2].fov, glm::smoothstep(0., 1., rt));
                let pos = solve_eq(rt, self[p0].pos, self[p1].pos, self[p2].pos, self[p3].pos);
                let focus = solve_eq(
                    rt,
                    self[p0].focus,
                    self[p1].focus,
                    self[p2].focus,
                    self[p3].focus,
                );
                let rot = solve_eq(rt, self[p0].rot, self[p1].rot, self[p2].rot, self[p3].rot);
                let vec = CameraSnapshot {
                    pos,
                    focus,
                    rot,
                    fov,
                };
                vec.set_inplace(gc);
                t += fraction;
                std::thread::sleep(sleep_duration);
            }

            if !loop_it {
                break;
            }
        }
    }
}
