//! Piloto automático: lleva la nave hacia un planeta usando los mismos controles que el
//! jugador (girar, acelerar, frenar). No teletransporta: la nave vuela de verdad, así que
//! también sirve para grabar el recorrido del video.
use crate::ship::{Controls, Ship};
use raylib::prelude::*;

const DECEL: f32 = 10.0; // con cuánta fuerza piensa frenar al llegar

/// Distancia al centro a la que se detiene.
pub fn stop_distance(radius: f32) -> f32 {
    radius * 1.8 + 2.0
}

/// Devuelve los controles para este paso y si ya llegó.
pub fn steer(ship: &Ship, target: Vector3, target_vel: Vector3, radius: f32) -> (Controls, bool) {
    let to_target = target - ship.pos;
    let dist = to_target.length();
    let remaining = dist - stop_distance(radius);
    // Apunta un poco adelante de donde va a estar el planeta.
    let lead = target_vel * (remaining.max(0.0) / 60.0).min(2.0);
    let aim = (to_target + lead).normalized();

    let (lx, ly, lz) = (aim.dot(ship.right()), aim.dot(ship.up), aim.dot(ship.forward));
    let mut c = Controls {
        yaw: (lx * 3.0).clamp(-1.0, 1.0),
        pitch: (ly * 3.0).clamp(-1.0, 1.0),
        ..Default::default()
    };
    if lz < 0.0 {
        c.yaw = if lx >= 0.0 { 1.0 } else { -1.0 };
    }

    let rel_vel = ship.vel - target_vel;
    let v_now = rel_vel.dot(to_target.normalized());
    let v_des = (2.0 * DECEL * remaining.max(0.0)).sqrt().min(120.0);
    if lz > 0.85 {
        c.thrust = ((v_des - v_now) / 6.0).clamp(-1.0, 1.0);
        c.boost = v_des > 50.0 && v_now < v_des - 5.0;
    }
    c.brake = v_now > v_des + 6.0 || (remaining < 5.0 && rel_vel.length() > 6.0);
    let arrived = remaining < 3.0 && rel_vel.length() < 9.0;
    (c, arrived)
}
