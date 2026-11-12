//! Recorrido automático para el video: `cargo run --release -- tour`.
//! Maneja la nave con un guion (los mismos controles que el teclado y el piloto automático)
//! y guarda cada cuadro en `docs/frames/`. Después `tools/make_video.sh` arma el video.
//! Como el tiempo avanza de 1/30 en 1/30 s, el video sale fluido aunque cada cuadro
//! tarde más en dibujarse.
use crate::framebuffer::Framebuffer;
use crate::game::{Game, Input, Mode};
use crate::math;
use crate::render::{Meshes, Renderer};
use crate::ship::Controls;
use noise::OpenSimplex;

const FPS: f32 = 30.0;

enum Step {
    /// Volar unos segundos con controles fijos.
    Fly(f32, Controls),
    /// Piloto automático hasta llegar (entra solo al modo local).
    Goto(usize),
    /// Rodear el planeta en modo local.
    Orbit(f32, Controls),
    /// En modo local, girar solo hacia la serpiente de roca y pasar junto a ella.
    Serpent(f32),
    /// En modo local, avanzar hacia el lado de día del planeta.
    Sunward(f32),
    /// Salir del modo local.
    Leave,
    /// Vista del sistema desde arriba.
    Overview(f32),
}

fn c(thrust: f32, yaw: f32, pitch: f32, lift: f32, boost: bool) -> Controls {
    Controls { thrust, yaw, pitch, lift, brake: false, boost }
}

fn script() -> Vec<Step> {
    vec![
        Step::Fly(3.0, c(0.6, 0.0, 0.0, 0.0, false)),
        Step::Fly(1.5, c(0.6, 0.5, 0.0, 0.0, false)),
        Step::Goto(2), // Tessara
        Step::Orbit(3.0, c(0.7, 0.0, 0.0, -0.6, false)),
        Step::Serpent(11.0),
        Step::Orbit(3.0, c(0.8, -0.25, 0.0, 0.0, false)),
        Step::Orbit(3.0, c(0.6, 0.0, 0.0, 1.0, false)),
        Step::Leave,
        Step::Goto(7), // Nix, la luna
        Step::Sunward(5.0),
        Step::Leave,
        Step::Fly(3.0, c(1.0, 0.0, 0.25, 0.0, true)),
        Step::Goto(6), // Mirra
        Step::Sunward(6.0),
        Step::Orbit(5.0, c(0.9, 0.3, 0.0, -0.3, false)),
        Step::Leave,
        Step::Fly(2.0, c(1.0, 0.0, 0.2, 0.0, true)),
        Step::Goto(5), // Orvane
        Step::Sunward(5.0),
        Step::Orbit(4.0, c(1.0, 0.15, 0.0, 0.2, false)),
        Step::Leave,
        Step::Fly(4.0, c(1.0, 0.0, 0.0, 0.0, true)),
        Step::Overview(5.0),
    ]
}

/// `save = false` solo simula (sirve para probar el guion rápido).
/// Controles para avanzar hacia la serpiente: gira el rumbo hacia donde está.
fn steer_to_serpent(game: &Game, meshes: &Meshes) -> Controls {
    let Mode::Local(lo) = game.mode else { return Controls::default() };
    let s = &meshes.serpent;
    let b = &game.system.bodies[s.body];
    let so = b.shade_object(game.orbit_time);
    let target = so.to_world(s.dir);
    let want = math::reject(target - lo.dir, lo.dir);
    let side = want.dot(lo.dir.cross(lo.heading)); // + = a la izquierda
    let dist = lo.dir.dot(target).clamp(-1.0, 1.0).acos();
    let mut ctl = c(if dist > 0.25 { 0.8 } else { 0.35 }, (-side * 6.0).clamp(-1.0, 1.0), 0.0, 0.0, false);
    // Pasa de lado (no encima) para que se vea la serpiente completa.
    if dist < 0.3 {
        ctl.yaw = (ctl.yaw + 0.6).clamp(-1.0, 1.0);
    }
    ctl
}

/// Controles para ir hacia el punto del planeta que mira al sol.
fn steer_to_sun(game: &Game) -> Controls {
    let Mode::Local(lo) = game.mode else { return Controls::default() };
    let center = game.system.bodies[lo.body].center(game.orbit_time);
    let target = (game.system.bodies[0].center(game.orbit_time) - center).normalized();
    let want = math::reject(target - lo.dir, lo.dir);
    let side = want.dot(lo.dir.cross(lo.heading));
    let dist = lo.dir.dot(target).clamp(-1.0, 1.0).acos();
    c(if dist > 0.5 { 1.0 } else { 0.4 }, (-side * 6.0).clamp(-1.0, 1.0), 0.0, -0.3, false)
}

pub fn run(width: usize, height: usize, save: bool) {
    let noise = OpenSimplex::new(7);
    let meshes = Meshes::build(&noise).unwrap_or_else(|e| panic!("{e}"));
    let renderer = Renderer::new(noise);
    let mut game = Game::new(noise);
    let mut fb = Framebuffer::new(width, height);
    let _ = std::fs::remove_dir_all("docs/frames");
    std::fs::create_dir_all("docs/frames").expect("docs/frames");
    let dt = 1.0 / FPS;
    let mut frame = 0usize;

    for step in script() {
        let mut elapsed = 0.0;
        let mut first = true;
        loop {
            let mut input = Input::default();
            let done = match &step {
                Step::Fly(secs, controls) | Step::Orbit(secs, controls) => {
                    input.controls = *controls;
                    elapsed >= *secs
                }
                Step::Serpent(secs) => {
                    input.controls = steer_to_serpent(&game, &meshes);
                    elapsed >= *secs
                }
                Step::Sunward(secs) => {
                    input.controls = steer_to_sun(&game);
                    elapsed >= *secs
                }
                Step::Goto(body) => {
                    if first {
                        input.autopilot = Some(*body);
                    }
                    matches!(game.mode, Mode::Local(lo) if lo.body == *body) || elapsed > 60.0
                }
                Step::Leave => {
                    input.toggle_local = matches!(game.mode, Mode::Local(_));
                    !first
                }
                Step::Overview(secs) => {
                    input.toggle_overview = first;
                    elapsed >= *secs
                }
            };
            if done && !first {
                break;
            }
            game.update(&input, dt);
            if save {
                renderer.render(&mut fb, &meshes, &game.frame());
                fb.save_png(&format!("docs/frames/{frame:05}.png"));
            }
            frame += 1;
            elapsed += dt;
            first = false;
        }
        let mode = match game.mode {
            Mode::Local(lo) => format!("local en {}", game.system.bodies[lo.body].name),
            Mode::Free => "libre".to_string(),
        };
        println!("paso listo: cuadro {frame} ({:.1} s), {mode}, velocidad {:.1}", frame as f32 / FPS, game.ship.speed());
    }
}
