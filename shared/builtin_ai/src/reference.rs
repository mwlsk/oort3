#![allow(non_upper_case_globals, non_snake_case)]
use oort_api::prelude::*;

const τ: f64 = TAU;
const π: f64 = PI;

pub struct Ship {
    target_position: Vec2,
    target_velocity: Vec2,
    last_contact_time: f64,
}

impl Ship {
    pub fn new() -> Ship {
        let (target_position, target_velocity) = parse_orders(receive());
        Ship {
            target_position,
            target_velocity,
            last_contact_time: current_time(),
        }
    }

    pub fn tick(&mut self) {
        if class() == Class::Missile {
            self.missile_tick();
        } else if class() == Class::Torpedo {
            self.torpedo_tick();
        } else {
            self.ship_tick();
        }
    }

    pub fn ship_tick(&mut self) {
        if class() == Class::Cruiser {
            if current_tick() % 6 == 0 {
                set_radar_width(τ);
            } else {
                set_radar_width(τ / 60.0);
                set_radar_heading(τ * (current_tick() as f64 * 2.0) / 60.0);
            }
        }

        let mut scan_result = scan();
        if scan_result.is_some() && scan_result.as_ref().unwrap().class == Class::Unknown {
            scan_result = None;
        }
        if let Some(contact) = scan_result.as_ref() {
            let Δp = contact.position - position();
            let Δv = contact.velocity - velocity();
            let mut predicted_dp = Δp;
            let bullet_speed = 1000.0;
            if Δp.dot(Δv) > -0.9 {
                for _ in 0..3 {
                    predicted_dp = Δp + Δv * predicted_dp.length() / bullet_speed;
                }
            }
            set_radar_heading(Δp.angle());
            self.target_position = contact.position;
            self.target_velocity = contact.velocity;

            if class() == Class::Fighter {
                if predicted_dp.length() < 5000.0 {
                    fire(0);
                }
                send(make_orders(contact.position, contact.velocity));
                fire(1);
            } else if class() == Class::Frigate {
                fire(0);
                aim(
                    1,
                    (predicted_dp - vec2(0.0, 15.0).rotate(heading())).angle(),
                );
                fire(1);
                aim(
                    2,
                    (predicted_dp - vec2(0.0, -15.0).rotate(heading())).angle(),
                );
                fire(2);
                send(make_orders(contact.position, contact.velocity));
                fire(3);
            } else if class() == Class::Cruiser {
                if predicted_dp.length() < 5000.0 {
                    aim(0, predicted_dp.angle());
                    fire(0);
                }
                for i in [1, 2] {
                    send(make_orders(contact.position, contact.velocity));
                    fire(i);
                }
                if contact.class == Class::Frigate || contact.class == Class::Cruiser {
                    send(make_orders(contact.position, contact.velocity));
                    fire(3);
                }
                //dbg.draw_diamond(contact.position, 30.0, 0xffff00);
            }
        } else {
            set_radar_heading(rand(0.0, τ));
            if (self.target_position - position()).length() < 100.0 {
                self.target_position = vec2(rand(3500.0, 4500.0), 0.0).rotate(rand(0.0, τ));
                self.target_velocity = vec2(0.0, 0.0);
            }
        }

        let Δp = self.target_position - position();
        let dist = Δp.length();
        let mut bullet_speed = 1000.0;
        if class() == Class::Frigate {
            bullet_speed = 4000.0;
        }
        let t = dist / bullet_speed;
        let predicted_dp = Δp + t * (self.target_velocity - velocity());
        self.turn_to(predicted_dp.angle());

        if scan_result.is_some() && dist < 1000.0 {
            accelerate(-velocity());
        } else {
            accelerate(Δp - velocity());
        }
    }

    fn missile_tick(&mut self) {
        let acc = max_forward_acceleration();

        if let Some(contact) = scan() {
            self.target_position = contact.position;
            self.target_velocity = contact.velocity;
            set_radar_width((radar_width() * 0.9).max(τ / 360.0));
            draw_diamond(self.target_position, 20.0, 0x00ff00);
        } else if receive().is_some() {
            let (new_target_position, new_target_velocity) = parse_orders(receive());
            if new_target_position.distance(self.target_position) < 100.0 {
                self.target_position = new_target_position;
                self.target_velocity = new_target_velocity;
                set_radar_width(τ / 360.0);
                draw_diamond(self.target_position, 20.0, 0xf5da42);
            } else {
                set_radar_width((radar_width() * 2.0).min(τ / 16.0));
                draw_diamond(self.target_position, 20.0, 0xff0000);
            }
        } else {
            set_radar_width((radar_width() * 2.0).min(τ / 16.0));
            draw_diamond(self.target_position, 20.0, 0xff0000);
        }

        set_radar_heading((self.target_position - position()).angle());

        let Δp = self.target_position - position();
        let Δv = self.target_velocity - velocity();

        let dist = Δp.length();
        let next_dist = (Δp + Δv / 60.0).length();
        if next_dist < 30.0 || dist < 100.0 && next_dist > dist {
            explode();
            return;
        }

        if dist < 300.0 {
            set_radar_width(τ / 6.0);
        }

        let badv = -(Δv - Δv.dot(Δp) * Δp.normalize() / Δp.length());
        let a = (Δp - badv * 10.0).normalize() * acc;
        accelerate(a);
        self.turn_to(a.angle());
    }

    fn torpedo_tick(&mut self) {
        let mut acc = max_forward_acceleration();
        self.target_velocity = velocity();

        let target_heading = (self.target_position - position()).angle();
        set_radar_heading(
            target_heading + rand(-π, π) * ((current_time() - self.last_contact_time) / 10.0),
        );
        if (self.target_position - position()).length() < 200.0 {
            set_radar_width(π * 2.0 / 6.0);
        } else {
            set_radar_width(π * 2.0 / 60.0);
        }

        let mut contact = scan();
        if contact.is_some()
            && class() == Class::Torpedo
            && contact.as_ref().unwrap().class != Class::Frigate
            && contact.as_ref().unwrap().class != Class::Cruiser
        {
            contact = None;
        }
        if let Some(contact) = &contact {
            self.target_position = contact.position;
            self.target_velocity = contact.velocity;
            self.last_contact_time = current_time();
        } else {
            self.target_position += self.target_velocity / 60.0;
        }

        let Δp = self.target_position - position();
        let Δv = self.target_velocity - velocity();

        if contact.is_some() {
            let dist = Δp.length();
            let next_dist = (Δp + Δv / 60.0).length();
            if next_dist < 60.0 || dist < 100.0 && next_dist > dist {
                explode();
                return;
            }
        } else {
            acc /= 2.0;
        }

        let predicted_position =
            self.target_position + self.target_velocity * (Δp.length() / 8000.0);
        let pdp = predicted_position - position();

        let badv = -(Δv - Δv.dot(Δp) * pdp.normalize() / pdp.length());
        let a = (pdp - badv * 10.0).normalize() * acc;
        accelerate(a);
        self.turn_to(a.angle());

        /*
        if no_contact_ticks > 0 {
            dbg.draw_diamond(target_position, 20.0, 0xff0000);
        } else {
            dbg.draw_diamond(contact.position, 20.0, 0xffff00);
            dbg.draw_diamond(position() + pdp, 5.0, 0xffffff);
        }

        dbg.draw_line(position(), position() + Δp, 0x222222);
        dbg.draw_line(position(), position() - Δv, 0xffffff);
        dbg.draw_line(position(), position() + badv, 0x222299);
        */
    }

    fn turn_to(&mut self, target_heading: f64) {
        let heading_error = angle_diff(heading(), target_heading);
        turn(10.0 * heading_error);
    }
}

fn parse_orders(msg: Option<Message>) -> (Vec2, Vec2) {
    if let Some(msg) = msg {
        (vec2(msg[0], msg[1]), vec2(msg[2], msg[3]))
    } else {
        (vec2(0.0, 0.0), vec2(0.0, 0.0))
    }
}

fn make_orders(p: Vec2, v: Vec2) -> Message {
    [p.x, p.y, v.x, v.y]
}