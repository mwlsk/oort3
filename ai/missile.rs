use crate::prelude::*;

pub struct Ship {}

impl Ship {
    pub fn new() -> Ship {
        Ship {}
    }

    pub fn tick(&mut self) {
        if let Some(contact) = scan() {
            seek(contact.position, contact.velocity);

            let dp = contact.position - position();
            let dv = contact.velocity - velocity();
            if dp.length().min((dp + dv * TICK_LENGTH).length()) < 25.0 {
                explode();
            }

            set_radar_heading((contact.position - position()).angle() - heading());
            set_radar_width((10.0 * TAU / dp.length()).clamp(TAU / 30.0, TAU));
        } else {
            set_radar_width(TAU);
        }
    }
}

pub fn seek(p: Vec2, v: Vec2) {
    let dp = p - position();
    let dv = v - velocity();
    let closing_speed = -(dp.y * dv.y - dp.x * dv.x).abs() / dp.length();
    let los = dp.angle();
    let los_rate = (dp.y * dv.x - dp.x * dv.y) / (dp.length() * dp.length());

    const N: f64 = 4.0;
    let a = vec2(100.0, N * closing_speed * los_rate).rotate(los);
    let a = vec2(400.0, 0.0).rotate(a.angle());
    accelerate(a.rotate(-heading()));
    turn_to(a.angle(), 0.0);
}

fn turn_to(target_heading: f64, target_angular_velocity: f64) {
    let acc = 2.0 * TAU;
    let dh = angle_diff(heading(), target_heading);
    let vh = angular_velocity() - target_angular_velocity;
    let t = (vh / acc).abs();
    let pdh = vh * t + 0.5 * -acc * t * t - dh;
    if pdh < 0.0 {
        torque(acc);
    } else if pdh > 0.0 {
        torque(-acc);
    }
}
