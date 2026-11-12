//! Cámara en tercera persona que sigue a la nave, y una vista general del sistema.
//! Las dos se mueven con suavizado exponencial (no dependen de los FPS).
use crate::math::{damp_v, rotate_axis};
use crate::render::{CameraView, FOV_Y};
use crate::ship::Ship;
use raylib::prelude::*;

pub struct ChaseCamera {
    pub view: CameraView,
    pub distance: f32,
    pub look_yaw: f32, // giro extra con el mouse alrededor de la nave
    pub look_pitch: f32,
    pub overview: bool,
    pub dragging: bool, // el mouse está girando la cámara
    pub local: bool,    // la nave está en modo local: la cámara mira más hacia el suelo
    pub altitude: f32,  // altura sobre el planeta en modo local
    pub shake: f32,     // sacudida al chocar (se apaga sola)
    pub overview_zoom: f32, // 1 = todo el sistema; menos = más cerca
    clock: f32,
}

impl ChaseCamera {
    pub fn new(ship: &Ship) -> Self {
        let mut c = Self {
            view: CameraView { eye: ship.pos, target: ship.pos + ship.forward, up: ship.up, fov: FOV_Y },
            distance: 4.2,
            look_yaw: 0.0,
            look_pitch: 0.0,
            overview: false,
            dragging: false,
            local: false,
            altitude: 0.0,
            shake: 0.0,
            overview_zoom: 1.0,
            clock: 0.0,
        };
        c.snap(ship);
        c
    }

    fn desired(&self, ship: &Ship) -> CameraView {
        if self.overview {
            // Vista desde arriba del plano eclíptico, girable con el mouse.
            let eye = rotate_axis(Vector3::new(0.0, 300.0, 250.0) * self.overview_zoom, Vector3::new(0.0, 1.0, 0.0), self.look_yaw);
            return CameraView { eye, target: Vector3::zero(), up: Vector3::new(0.0, 1.0, 0.0), fov: FOV_Y };
        }
        let right = ship.right();
        // En modo local la cámara sube y mira hacia abajo, para que el planeta llene la vista.
        // Entre más alto va, más baja la mirada para no perder el planeta de vista.
        let (height, drop) = if self.local { (0.6, 1.1 + self.altitude * 0.45) } else { (0.3, -0.4) };
        let mut back = -ship.forward * (self.distance * 0.9) + ship.up * (self.distance * height);
        back = rotate_axis(back, ship.up, self.look_yaw);
        back = rotate_axis(back, right, self.look_pitch);
        CameraView {
            eye: ship.pos + back,
            target: ship.pos + ship.forward * 2.5 - ship.up * drop,
            up: ship.up,
            // Con turbo la apertura crece un poco: da sensación de velocidad.
            fov: FOV_Y + ((ship.speed() - 45.0) / 85.0).clamp(0.0, 1.0) * 14.0,
        }
    }

    pub fn snap(&mut self, ship: &Ship) {
        self.view = self.desired(ship);
    }

    /// `ground` = (centro, radio) del planeta más cercano, para que la cámara no se meta en él.
    pub fn update(&mut self, ship: &Ship, ground: Option<(Vector3, f32)>, dt: f32) {
        let d = self.desired(ship);
        let rate = if self.overview { 3.0 } else { 7.0 };
        self.view.eye = damp_v(self.view.eye, d.eye, rate, dt);
        self.view.target = damp_v(self.view.target, d.target, 12.0, dt);
        self.view.up = damp_v(self.view.up, d.up, 4.0, dt).normalized();
        self.view.fov = crate::math::damp(self.view.fov, d.fov, 2.0, dt);
        if let (Some((c, r)), false) = (ground, self.overview) {
            let off = self.view.eye - c;
            if off.length() < r + 0.25 {
                self.view.eye = c + off.normalized() * (r + 0.25);
            }
        }
        // Sacudida: senos con frecuencias distintas en cada eje, que se apagan rápido.
        self.clock += dt;
        if self.shake > 0.01 {
            let c = self.clock;
            let jitter = Vector3::new((c * 53.0).sin(), (c * 71.0).sin(), (c * 37.0).sin());
            self.view.eye += jitter * (self.shake * 0.12);
            self.shake *= (-5.0 * dt).exp();
        }
        // Al soltar el mouse la cámara regresa sola detrás de la nave.
        if !self.dragging && !self.overview {
            self.look_yaw *= (-1.5 * dt).exp();
            self.look_pitch *= (-1.5 * dt).exp();
        }
    }
}
