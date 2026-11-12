//! La nave: posición, velocidad y orientación (adelante + arriba).
//!
//! Todo es progresivo: la entrada del teclado no cambia la velocidad de golpe sino el
//! empuje y la velocidad de giro, que se acercan a su valor objetivo con un suavizado
//! exponencial. La física corre con paso fijo (ver `game.rs`), así la nave responde igual
//! en una computadora lenta que en una rápida.
use crate::math::{self, Mat4, damp, rotate_axis};
use raylib::prelude::*;

pub const SCALE: f32 = 0.16; // la nave.obj mide ~8 unidades: queda de ~1.3
pub const RADIUS: f32 = 0.7; // radio para las colisiones
const THRUST: f32 = 20.0; // aceleración con el acelerador a fondo
const BOOST: f32 = 3.0;
const LIFT: f32 = 10.0;
const MAX_SPEED: f32 = 45.0;
const MAX_BOOST_SPEED: f32 = 130.0;
const DRAG: f32 = 0.12;
const BRAKE_DRAG: f32 = 2.2;
const TURN_RATE: f32 = 1.5; // rad/s
const PITCH_RATE: f32 = 1.2;
const RESPONSE: f32 = 4.0; // qué tan rápido llega el giro a su objetivo

/// Entrada ya interpretada (teclado o piloto automático). Valores en [-1, 1].
#[derive(Clone, Copy, Debug, Default)]
pub struct Controls {
    pub thrust: f32,
    pub yaw: f32,   // + = derecha
    pub pitch: f32, // + = nariz arriba
    pub lift: f32,  // + = subir
    pub brake: bool,
    pub boost: bool,
}

impl Controls {
    pub fn is_idle(&self) -> bool {
        self.thrust == 0.0 && self.yaw == 0.0 && self.pitch == 0.0 && self.lift == 0.0 && !self.brake
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ship {
    pub pos: Vector3,
    pub vel: Vector3,
    pub forward: Vector3,
    pub up: Vector3,
    pub yaw_rate: f32,
    pub pitch_rate: f32,
    pub throttle: f32,
    pub bank: f32, // inclinación visual al girar
}

impl Ship {
    pub fn new(pos: Vector3, forward: Vector3) -> Self {
        let forward = forward.normalized();
        let up = math::reject(Vector3::new(0.0, 1.0, 0.0), forward).normalized();
        Self { pos, vel: Vector3::zero(), forward, up, yaw_rate: 0.0, pitch_rate: 0.0, throttle: 0.0, bank: 0.0 }
    }

    /// Derecha vista desde atrás de la nave.
    pub fn right(&self) -> Vector3 {
        self.forward.cross(self.up).normalized()
    }

    pub fn speed(&self) -> f32 {
        self.vel.length()
    }

    /// Un paso de física en vuelo libre. `gravity` = aceleración que causan los planetas.
    pub fn step(&mut self, c: &Controls, gravity: Vector3, dt: f32) {
        // Giro progresivo: la velocidad angular se acerca al objetivo, no salta.
        self.yaw_rate = damp(self.yaw_rate, -c.yaw * TURN_RATE, RESPONSE, dt);
        self.pitch_rate = damp(self.pitch_rate, c.pitch * PITCH_RATE, RESPONSE, dt);
        self.forward = rotate_axis(self.forward, self.up, self.yaw_rate * dt);
        let right = self.right();
        self.forward = rotate_axis(self.forward, right, self.pitch_rate * dt).normalized();
        self.up = rotate_axis(self.up, right, self.pitch_rate * dt);

        // Nivelado automático: el "arriba" de la nave vuelve despacio al del plano eclíptico.
        let level = math::reject(Vector3::new(0.0, 1.0, 0.0), self.forward);
        if level.length() > 0.2 {
            self.up = math::damp_v(self.up, level.normalized(), 1.2, dt);
        }
        self.up = math::reject(self.up, self.forward).normalized();

        // Empuje progresivo.
        self.throttle = damp(self.throttle, c.thrust, 2.5, dt);
        let boost = if c.boost && c.thrust > 0.0 { BOOST } else { 1.0 };
        let acc = self.forward * (self.throttle * THRUST * boost) + self.up * (c.lift * LIFT) + gravity;
        self.vel += acc * dt;

        // Mientras acelera, la velocidad lateral se va perdiendo (así se puede dar la vuelta).
        // Sin acelerar no: ahí se nota cómo la gravedad dobla la trayectoria.
        if self.throttle.abs() > 0.1 {
            let along = self.forward * self.vel.dot(self.forward);
            let lateral = self.vel - along;
            self.vel = along + lateral * (-1.5 * self.throttle.abs() * dt).exp();
        }
        let drag = DRAG + if c.brake { BRAKE_DRAG } else { 0.0 };
        self.vel *= (-drag * dt).exp();
        let max = if c.boost { MAX_BOOST_SPEED } else { MAX_SPEED.max(self.speed() * 0.995) };
        if self.speed() > max {
            self.vel = self.vel.normalized() * max;
        }
        self.pos += self.vel * dt;
        self.bank = damp(self.bank, -self.yaw_rate * 0.45, 5.0, dt);
    }

    /// Matriz de modelo con la inclinación visual al girar.
    pub fn model_matrix(&self) -> Mat4 {
        let up = rotate_axis(self.up, self.forward, self.bank);
        let right = up.cross(self.forward);
        math::from_basis(self.pos, right, up, self.forward, SCALE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nave() -> Ship {
        Ship::new(Vector3::zero(), Vector3::new(0.0, 0.0, 1.0))
    }

    fn volar(ship: &mut Ship, c: Controls, secs: f32, dt: f32) {
        for _ in 0..(secs / dt).round() as usize {
            ship.step(&c, Vector3::zero(), dt);
        }
    }

    #[test]
    fn acelerar_es_progresivo() {
        let mut s = nave();
        let c = Controls { thrust: 1.0, ..Default::default() };
        volar(&mut s, c, 0.1, 1.0 / 120.0);
        let v1 = s.speed();
        volar(&mut s, c, 0.9, 1.0 / 120.0);
        // No salta a la velocidad máxima: crece de a poco.
        assert!(v1 < 1.0 && s.speed() > v1 * 5.0);
    }

    #[test]
    fn frenar_detiene_la_nave() {
        let mut s = nave();
        volar(&mut s, Controls { thrust: 1.0, ..Default::default() }, 3.0, 1.0 / 120.0);
        let antes = s.speed();
        volar(&mut s, Controls { brake: true, ..Default::default() }, 2.0, 1.0 / 120.0);
        assert!(s.speed() < antes * 0.05);
    }

    #[test]
    fn girar_a_la_derecha() {
        let mut s = nave();
        volar(&mut s, Controls { yaw: 1.0, ..Default::default() }, 0.5, 1.0 / 120.0);
        // Mirando hacia +Z con Y arriba, la derecha es -X.
        assert!(s.forward.x < -0.1);
        assert!((s.forward.length() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn la_base_sigue_ortonormal() {
        let mut s = nave();
        let c = Controls { thrust: 0.5, yaw: 0.7, pitch: -0.6, ..Default::default() };
        volar(&mut s, c, 5.0, 1.0 / 120.0);
        assert!(s.forward.dot(s.up).abs() < 1e-3);
        assert!((s.up.length() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn la_gravedad_dobla_la_trayectoria() {
        let mut s = nave();
        s.vel = Vector3::new(0.0, 0.0, 10.0);
        for _ in 0..120 {
            s.step(&Controls::default(), Vector3::new(3.0, 0.0, 0.0), 1.0 / 120.0);
        }
        assert!(s.vel.x > 2.5);
    }

    #[test]
    fn no_pasa_la_velocidad_maxima() {
        let mut s = nave();
        volar(&mut s, Controls { thrust: 1.0, ..Default::default() }, 30.0, 1.0 / 120.0);
        assert!(s.speed() <= MAX_SPEED + 1e-3);
    }
}
