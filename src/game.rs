//! Estado del juego y su actualización: tiempo, órbitas, nave, gravedad, colisiones,
//! modo local, piloto automático y los parámetros que se cambian en vivo.
use crate::autopilot;
use crate::camera::ChaseCamera;
use crate::decor::{self, Instance};
use crate::local_orbit::{self, LocalOrbit};
use crate::render::{Frame, Settings};
use crate::ship::{self, Controls, Ship};
use crate::system::System;
use noise::OpenSimplex;
use raylib::prelude::*;

/// Paso fijo de la física: 120 veces por segundo, sin importar los FPS.
pub const STEP: f32 = 1.0 / 120.0;

#[derive(Clone, Copy, Debug)]
pub enum Mode {
    Free,
    Local(LocalOrbit),
}

/// Lo que pidió el jugador en este cuadro (ya leído del teclado y el mouse).
#[derive(Default, Clone, Copy)]
pub struct Input {
    pub controls: Controls,
    pub toggle_local: bool,
    pub autopilot: Option<usize>,
    pub toggle_overview: bool,
    pub toggle_gravity: bool,
    pub toggle_pause: bool,
    pub toggle_culling: bool,
    pub toggle_lod: bool,
    pub toggle_normals: bool,
    pub toggle_orbits: bool,
    pub toggle_minimap: bool,
    pub seed_delta: i32,
    pub time_scale_delta: f32,
    pub radius_delta: f32,
    pub intensity_delta: f32,
    pub look: (f32, f32),
    pub dragging: bool,
    pub zoom: f32,
}

pub struct Game {
    pub system: System,
    pub ship: Ship,
    pub mode: Mode,
    pub camera: ChaseCamera,
    pub orbit_time: f32, // tiempo de las órbitas (se puede acelerar o pausar)
    pub time: f32,       // tiempo de los shaders
    pub time_scale: f32,
    pub paused: bool,
    pub gravity_on: bool,
    pub gravity: Vector3, // aceleración que siente la nave (para el HUD)
    pub autopilot: Option<usize>,
    pub decor_seed: u64,
    pub decor: Vec<Vec<Instance>>,
    pub settings: Settings,
    pub collided: f32, // tiempo desde el último choque (para avisar en el HUD)
    accumulator: f32,
    noise: OpenSimplex,
}

impl Game {
    pub fn new(noise: OpenSimplex) -> Self {
        let system = System::new();
        // Empieza cerca de Tessara, mirando hacia ella con el sol de lado.
        let tessara = system.bodies[2].center(0.0);
        let start = tessara + Vector3::new(-30.0, 2.5, -24.0);
        let ship = Ship::new(start, tessara - start);
        let camera = ChaseCamera::new(&ship);
        let mut g = Self {
            system,
            ship,
            mode: Mode::Free,
            camera,
            orbit_time: 0.0,
            time: 0.0,
            time_scale: 1.0,
            paused: false,
            gravity_on: true,
            gravity: Vector3::zero(),
            autopilot: None,
            decor_seed: 7,
            decor: Vec::new(),
            settings: Settings::default(),
            collided: 10.0,
            accumulator: 0.0,
            noise,
        };
        g.scatter_decor();
        g
    }

    pub fn scatter_decor(&mut self) {
        self.decor = self.system.bodies.iter().map(|b| decor::scatter(b, &self.noise, self.decor_seed)).collect();
    }

    /// Planeta al que se le cambian los parámetros: el del modo local, o el más cercano.
    pub fn selected(&self) -> usize {
        match self.mode {
            Mode::Local(lo) => lo.body,
            Mode::Free => self.system.nearest(self.ship.pos, self.orbit_time).0,
        }
    }

    pub fn update(&mut self, input: &Input, dt: f32) {
        self.apply_toggles(input);
        let dt = dt.min(0.1); // si la ventana se congela, que no salte todo

        self.accumulator += dt;
        while self.accumulator >= STEP {
            self.accumulator -= STEP;
            if !self.paused {
                self.orbit_time += STEP * self.time_scale;
                self.time += STEP;
            }
            self.physics(&input.controls, STEP);
        }
        self.collided += dt;

        let (nearest, _) = self.system.nearest(self.ship.pos, self.orbit_time);
        let b = &self.system.bodies[nearest];
        let ground = Some((b.center(self.orbit_time), b.radius() * 0.98));
        self.camera.dragging = input.dragging;
        self.camera.local = matches!(self.mode, Mode::Local(_));
        if let Mode::Local(lo) = self.mode {
            self.camera.altitude = lo.alt;
        }
        self.camera.look_yaw += input.look.0;
        self.camera.look_pitch = (self.camera.look_pitch + input.look.1).clamp(-1.2, 1.2);
        self.camera.distance = (self.camera.distance - input.zoom * 0.4).clamp(2.2, 14.0);
        self.camera.update(&self.ship, ground, dt);
    }

    fn apply_toggles(&mut self, input: &Input) {
        let s = &mut self.settings;
        s.culling ^= input.toggle_culling;
        s.lod ^= input.toggle_lod;
        s.normal_mapping ^= input.toggle_normals;
        s.orbits ^= input.toggle_orbits;
        s.minimap ^= input.toggle_minimap;
        self.paused ^= input.toggle_pause;
        self.gravity_on ^= input.toggle_gravity;
        self.camera.overview ^= input.toggle_overview;
        self.time_scale = (self.time_scale + input.time_scale_delta).clamp(0.0, 8.0);

        if input.seed_delta != 0 {
            self.decor_seed = (self.decor_seed as i64 + input.seed_delta as i64).max(0) as u64;
            self.scatter_decor();
        }
        let sel = self.selected();
        if input.radius_delta != 0.0 {
            let b = &mut self.system.bodies[sel];
            b.radius_scale = (b.radius_scale + input.radius_delta).clamp(0.5, 2.0);
        }
        if input.intensity_delta != 0.0 {
            let b = &mut self.system.bodies[sel];
            b.intensity = (b.intensity + input.intensity_delta).clamp(0.0, 1.0);
        }
        if let Some(i) = input.autopilot {
            self.autopilot = Some(i);
            if let Mode::Local(_) = self.mode {
                self.exit_local();
            }
        }
        if input.toggle_local {
            match self.mode {
                Mode::Local(_) => self.exit_local(),
                Mode::Free => {
                    let (i, d) = self.system.nearest(self.ship.pos, self.orbit_time);
                    let r = self.system.bodies[i].radius();
                    if d < local_orbit::max_altitude(r) {
                        self.enter_local(i);
                    }
                }
            }
        }
    }

    pub fn enter_local(&mut self, i: usize) {
        let t = self.orbit_time;
        let b = &self.system.bodies[i];
        let c = b.center(t);
        let surf = b.surface_radius(&self.noise, (self.ship.pos - c).normalized(), t);
        let mut lo = LocalOrbit::enter(i, &self.ship, c, surf, b.velocity(t, self.time_scale));
        lo.alt = lo.alt.clamp(local_orbit::min_altitude(b.radius()), local_orbit::max_altitude(b.radius()));
        self.mode = Mode::Local(lo);
        self.autopilot = None;
    }

    pub fn exit_local(&mut self) {
        // La nave ya tiene la posición y velocidad del modo local: solo se suelta.
        self.mode = Mode::Free;
    }

    /// Aceleración de gravedad en `p`: suma de GM / d² hacia cada cuerpo.
    fn gravity_at(&self, p: Vector3) -> Vector3 {
        let t = self.orbit_time;
        self.system.bodies.iter().fold(Vector3::zero(), |acc, b| {
            let d = b.center(t) - p;
            let dist = d.length().max(b.radius());
            acc + d * (b.gm() / (dist * dist * dist))
        })
    }

    fn physics(&mut self, user: &Controls, dt: f32) {
        let t = self.orbit_time;
        let mut controls = *user;
        if !user.is_idle() {
            self.autopilot = None; // cualquier tecla de vuelo cancela el piloto automático
        }
        match self.mode {
            Mode::Free => {
                if let Some(i) = self.autopilot {
                    let b = &self.system.bodies[i];
                    let (c, arrived) = autopilot::steer(&self.ship, b.center(t), b.velocity(t, self.time_scale), b.radius());
                    controls = c;
                    if arrived {
                        self.enter_local(i);
                        return;
                    }
                }
                self.gravity = if self.gravity_on { self.gravity_at(self.ship.pos) } else { Vector3::zero() };
                self.ship.step(&controls, self.gravity, dt);
                self.collide();
            }
            Mode::Local(mut lo) => {
                let b = &self.system.bodies[lo.body];
                let r = b.radius();
                lo.step(&controls, r, dt);
                let c = b.center(t);
                let surf = b.surface_radius(&self.noise, lo.dir, t);
                lo.apply(&mut self.ship, c, surf, b.velocity(t, self.time_scale), dt);
                self.gravity = Vector3::zero();
                self.mode = if lo.wants_exit(r) { Mode::Free } else { Mode::Local(lo) };
            }
        }
    }

    /// Colisión básica: si la nave entra en un cuerpo se empuja a la superficie y se le
    /// quita la velocidad que la metía (con un rebote pequeño).
    fn collide(&mut self) {
        let t = self.orbit_time;
        for b in &self.system.bodies {
            let c = b.center(t);
            let off = self.ship.pos - c;
            let dist = off.length();
            if dist > b.radius() * 1.3 + ship::RADIUS {
                continue;
            }
            let n = off * (1.0 / dist.max(1e-4));
            let surf = b.surface_radius(&self.noise, n, t) + ship::RADIUS;
            if dist < surf {
                self.ship.pos = c + n * surf;
                let bv = b.velocity(t, self.time_scale);
                let rel = self.ship.vel - bv;
                let into = rel.dot(n);
                if into < 0.0 {
                    self.ship.vel = bv + (rel - n * (into * 1.3)) * 0.85;
                }
                self.collided = 0.0;
            }
        }
    }

    pub fn frame(&self) -> Frame<'_> {
        Frame {
            system: &self.system,
            orbit_time: self.orbit_time,
            time: self.time,
            decor: &self.decor,
            ship_model: Some(self.ship.model_matrix()),
            ship_thrust: self.ship.throttle.max(0.0),
            ship_pos: self.ship.pos,
            ship_forward: self.ship.forward,
            camera: self.camera.view,
            settings: self.settings,
            highlight: self.autopilot.or(match self.mode {
                Mode::Local(lo) => Some(lo.body),
                Mode::Free => None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_fisica_no_depende_de_los_fps() {
        // Simular 3 s a 30 FPS y a 144 FPS tiene que dejar la nave casi en el mismo lugar.
        let run = |fps: f32| {
            let mut g = Game::new(OpenSimplex::new(7));
            let mut input = Input::default();
            input.controls.thrust = 1.0;
            input.controls.yaw = 0.4;
            for _ in 0..(3.0 * fps) as usize {
                g.update(&input, 1.0 / fps);
            }
            g.ship.pos
        };
        assert!((run(30.0) - run(144.0)).length() < 0.6);
    }

    #[test]
    fn la_nave_no_atraviesa_el_planeta() {
        let mut g = Game::new(OpenSimplex::new(7));
        let b = &g.system.bodies[2];
        let c = b.center(0.0);
        g.ship = Ship::new(c + Vector3::new(0.0, 0.0, -20.0), Vector3::new(0.0, 0.0, 1.0));
        g.ship.vel = Vector3::new(0.0, 0.0, 40.0);
        let mut input = Input::default();
        input.controls.thrust = 1.0;
        for _ in 0..90 {
            g.update(&input, 1.0 / 30.0);
            let b = &g.system.bodies[2];
            assert!((g.ship.pos - b.center(g.orbit_time)).length() >= b.radius());
        }
    }

    #[test]
    fn el_piloto_automatico_llega_a_cada_planeta() {
        for target in 1..=6 {
            let mut g = Game::new(OpenSimplex::new(7));
            let mut input = Input { autopilot: Some(target), ..Default::default() };
            for _ in 0..30 * 90 {
                g.update(&input, 1.0 / 30.0);
                input.autopilot = None;
                if matches!(g.mode, Mode::Local(lo) if lo.body == target) {
                    break;
                }
            }
            assert!(matches!(g.mode, Mode::Local(lo) if lo.body == target), "no llegó a {}", g.system.bodies[target].name);
        }
    }
}
