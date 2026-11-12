//! Modo de control local: sirve para rodear un planeta cercano sin chocar ni perderlo.
//!
//! La nave se "engancha" al planeta: su posición se guarda como una dirección desde el
//! centro, un rumbo tangente y una altura. W/S avanzan sobre la superficie siguiendo un
//! círculo máximo, A/D cambian el rumbo y Q/E (o las flechas) suben y bajan.
//! El planeta sigue orbitando al sol y la nave va con él. Al entrar y al salir se
//! conservan la posición y la velocidad, así el vuelo nunca se corta.
use crate::math::{self, damp, rotate_axis};
use crate::ship::{Controls, Ship};
use raylib::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct LocalOrbit {
    pub body: usize,
    pub dir: Vector3,     // del centro del planeta hacia la nave (unitario)
    pub heading: Vector3, // hacia dónde avanza, tangente a la superficie
    pub alt: f32,         // altura sobre la superficie
    pub speed: f32,
    pub turn: f32,
    pub climb: f32,
}

pub fn min_altitude(radius: f32) -> f32 {
    0.5 + radius * 0.03
}

pub fn max_altitude(radius: f32) -> f32 {
    radius * 5.0
}

impl LocalOrbit {
    /// Convierte el estado de la nave (vuelo libre) a coordenadas del planeta.
    pub fn enter(body: usize, ship: &Ship, center: Vector3, surface_r: f32, body_vel: Vector3) -> Self {
        let dir = (ship.pos - center).normalized();
        let mut heading = math::reject(ship.forward, dir);
        if heading.length() < 0.05 {
            heading = math::reject(ship.up, dir);
        }
        let heading = heading.normalized();
        let rel = ship.vel - body_vel;
        Self {
            body,
            dir,
            heading,
            alt: (ship.pos - center).length() - surface_r,
            speed: rel.dot(heading),
            turn: 0.0,
            climb: rel.dot(dir),
        }
    }

    pub fn step(&mut self, c: &Controls, radius: f32, dt: f32) {
        // Velocidad sobre la superficie: más lenta cerca del suelo para poder ver los detalles.
        let cruise = 6.0 + self.alt * 1.5;
        let boost = if c.boost { 2.5 } else { 1.0 };
        let target = if c.brake { 0.0 } else { c.thrust * cruise * boost };
        self.speed = damp(self.speed, target, if c.brake { 4.0 } else { 1.8 }, dt);
        self.turn = damp(self.turn, -c.yaw * 1.4, 4.0, dt);
        self.climb = damp(self.climb, (c.lift + c.pitch) * (3.0 + self.alt * 0.8), 3.0, dt);

        self.heading = rotate_axis(self.heading, self.dir, self.turn * dt);
        self.alt = (self.alt + self.climb * dt).clamp(min_altitude(radius), max_altitude(radius) + 1.0);
        if self.alt <= min_altitude(radius) && self.climb < 0.0 {
            self.climb = 0.0;
        }

        // Avanzar = girar la dirección (y el rumbo) alrededor del eje dir × rumbo.
        let r = radius + self.alt;
        let axis = self.dir.cross(self.heading).normalized();
        let angle = self.speed * dt / r;
        self.dir = rotate_axis(self.dir, axis, angle).normalized();
        self.heading = math::reject(rotate_axis(self.heading, axis, angle), self.dir).normalized();
    }

    pub fn wants_exit(&self, radius: f32) -> bool {
        self.alt > max_altitude(radius)
    }

    /// Pone la nave donde dice el modo local. `surface_r` = radio del suelo bajo la nave.
    pub fn apply(&self, ship: &mut Ship, center: Vector3, surface_r: f32, body_vel: Vector3, dt: f32) {
        ship.pos = center + self.dir * (surface_r + self.alt);
        ship.forward = self.heading;
        ship.up = self.dir;
        ship.vel = body_vel + self.heading * self.speed + self.dir * self.climb;
        ship.yaw_rate = self.turn;
        ship.pitch_rate = 0.0;
        ship.throttle = (self.speed.abs() / 12.0).min(1.0);
        ship.bank = damp(ship.bank, -self.turn * 0.4, 5.0, dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ship::Ship;

    fn orbita() -> LocalOrbit {
        let ship = Ship::new(Vector3::new(0.0, 8.0, 0.0), Vector3::new(0.0, 0.0, 1.0));
        LocalOrbit::enter(1, &ship, Vector3::zero(), 6.0, Vector3::zero())
    }

    #[test]
    fn avanzar_da_la_vuelta_al_planeta() {
        let mut lo = orbita();
        let inicio = lo.dir;
        let c = Controls { thrust: 1.0, ..Default::default() };
        let mut minimo: f32 = 1.0;
        for _ in 0..120 * 30 {
            lo.step(&c, 6.0, 1.0 / 120.0);
            minimo = minimo.min(lo.dir.dot(inicio));
        }
        // En algún momento estuvo del otro lado del planeta.
        assert!(minimo < -0.9);
    }

    #[test]
    fn nunca_baja_del_suelo() {
        let mut lo = orbita();
        let c = Controls { lift: -1.0, thrust: 0.5, ..Default::default() };
        for _ in 0..120 * 10 {
            lo.step(&c, 6.0, 1.0 / 120.0);
        }
        assert!(lo.alt >= min_altitude(6.0) - 1e-4);
    }

    #[test]
    fn el_rumbo_sigue_tangente() {
        let mut lo = orbita();
        let c = Controls { thrust: 1.0, yaw: 0.6, lift: 0.3, ..Default::default() };
        for _ in 0..120 * 8 {
            lo.step(&c, 6.0, 1.0 / 120.0);
        }
        assert!(lo.heading.dot(lo.dir).abs() < 1e-3);
        assert!((lo.dir.length() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn subir_mucho_suelta_la_nave() {
        let mut lo = orbita();
        let c = Controls { lift: 1.0, ..Default::default() };
        for _ in 0..120 * 30 {
            lo.step(&c, 6.0, 1.0 / 120.0);
        }
        assert!(lo.wants_exit(6.0));
    }
}
