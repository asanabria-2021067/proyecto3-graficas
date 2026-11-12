//! Fondo del espacio: la franja de la galaxia, nebulosas de colores y estrellas. Las dos dependen solo de hacia dónde
//! mira la cámara (están "en el infinito"), así que al girar la nave el cielo gira con ella.
use crate::framebuffer::Framebuffer;
use crate::math::{self, Mat4};
use crate::rng::Rng;
use crate::shaders::noise_helpers::fbm;
use crate::shaders::{rgb, smoothstep};
use noise::OpenSimplex;
use raylib::prelude::*;
use rayon::prelude::*;
use std::f32::consts::{PI, TAU};

const NEB_W: usize = 1024;
const NEB_H: usize = 512;

struct Star {
    dir: Vector3,
    color: Vector3,
    big: bool,
    twinkle: f32, // fase del parpadeo
}

pub struct Background {
    nebula: Vec<Vector3>, // textura equirectangular generada al inicio
    stars: Vec<Star>,
}

fn dir_from_uv(u: f32, v: f32) -> Vector3 {
    let lon = u * TAU - PI;
    let lat = (0.5 - v) * PI;
    Vector3::new(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin())
}

/// Plano de la galaxia: la franja de estrellas está inclinada respecto al plano eclíptico.
fn galaxy_normal() -> Vector3 {
    Vector3::new(0.3, 1.0, 0.2).normalized()
}

/// Nebulosas de colores: (centro en el cielo, color, tamaño angular).
fn nebulae() -> [(Vector3, Vector3, f32); 7] {
    [
        (Vector3::new(0.8, 0.15, -0.5), rgb(0.60, 0.12, 0.42), 0.6),    // magenta
        (Vector3::new(-0.7, 0.3, 0.6), rgb(0.06, 0.42, 0.52), 0.65),    // turquesa
        (Vector3::new(0.1, -0.5, 0.85), rgb(0.62, 0.26, 0.07), 0.5),    // naranja
        (Vector3::new(-0.3, 0.75, -0.6), rgb(0.26, 0.18, 0.66), 0.55),  // violeta
        (Vector3::new(-0.9, -0.35, -0.3), rgb(0.55, 0.10, 0.18), 0.4),  // rojo
        (Vector3::new(0.25, -0.9, -0.3), rgb(0.12, 0.30, 0.62), 0.6),   // azul (se ve desde la vista del sistema)
        (Vector3::new(0.5, 0.6, 0.6), rgb(0.15, 0.50, 0.40), 0.35),     // verde agua
    ]
}

fn nebula_color(noise: &OpenSimplex, d: Vector3) -> Vector3 {
    let mut c = rgb(0.003, 0.004, 0.010);

    // Franja de la galaxia: su ancho cambia con ruido y unas nubes de polvo oscuro la cortan.
    let lat = d.dot(galaxy_normal()) + fbm(noise, d * 2.0 + Vector3::new(7.0, 0.0, 0.0), 3) * 0.08;
    let band = (-(lat / 0.24).powi(2)).exp();
    if band > 0.01 {
        let glow = smoothstep(0.25, 0.85, fbm(noise, d * 3.5 + Vector3::new(40.0, 0.0, 0.0), 5) * 0.5 + 0.5);
        let dust = smoothstep(-0.05, 0.3, fbm(noise, d * 5.0 + Vector3::new(0.0, 50.0, 0.0), 5));
        let core = (-(lat / 0.07).powi(2)).exp() * (0.4 + glow);
        let light = rgb(0.07, 0.065, 0.10) * (0.3 + glow * 1.2) + rgb(0.14, 0.11, 0.08) * core;
        c += light * band * (1.0 - dust * (0.5 + core * 0.4));
    }

    // Nebulosas: filamentos de fBm deformado (domain warping) dentro de una mancha suave.
    for (center, color, size) in nebulae() {
        let ang = d.dot(center.normalized()).clamp(-1.0, 1.0).acos();
        let w = (-(ang / size).powi(2)).exp();
        if w < 0.02 {
            continue;
        }
        let seed = center * 13.0;
        let warp = Vector3::new(fbm(noise, d * 2.5 + seed, 3), fbm(noise, d * 2.5 + seed + Vector3::new(5.2, 1.3, 0.0), 3), 0.0);
        let f = fbm(noise, d * 4.0 + seed + warp * 1.8, 5) * 0.5 + 0.5;
        let filaments = smoothstep(0.42, 0.85, f).powf(1.5);
        let hot = smoothstep(0.7, 0.95, f) * w; // centros más claros
        c += color * (w * (0.1 + filaments * 1.8)) + rgb(0.9, 0.85, 0.95) * hot * 0.2;
    }
    c
}

impl Background {
    pub fn new(noise: &OpenSimplex) -> Self {
        // La textura de la nebulosa se calcula una sola vez al iniciar.
        let nebula = (0..NEB_W * NEB_H)
            .into_par_iter()
            .map(|i| nebula_color(noise, dir_from_uv((i % NEB_W) as f32 / NEB_W as f32, (i / NEB_W) as f32 / NEB_H as f32)))
            .collect();

        // Estrellas: la mitad repartidas en todo el cielo y la otra mitad apretadas cerca de
        // la franja de la galaxia (se aceptan con más probabilidad mientras más cerca estén).
        let mut rng = Rng::new(2026);
        let mut stars = Vec::with_capacity(12_000);
        while stars.len() < 12_000 {
            let dir = rng.on_sphere();
            let near_band = (-(dir.dot(galaxy_normal()) / 0.25).powi(2)).exp();
            if stars.len() % 2 == 1 && rng.f32() > near_band {
                continue;
            }
            let b = rng.f32().powf(7.0); // casi todas tenues, pocas brillantes
            let kind = rng.f32();
            let tint = if kind < 0.6 {
                rgb(0.75, 0.82, 1.0)
            } else if kind < 0.85 {
                rgb(1.0, 0.92, 0.78)
            } else if kind < 0.95 {
                rgb(1.0, 0.7, 0.5)
            } else {
                rgb(0.6, 0.7, 1.0)
            };
            stars.push(Star { dir, color: tint * (0.12 + b * 1.1), big: b > 0.45, twinkle: rng.range(0.0, TAU) });
        }
        Self { nebula, stars }
    }

    /// Color de la nebulosa en una dirección (lectura bilineal de la textura).
    pub fn sample(&self, d: Vector3) -> Vector3 {
        let u = (d.z.atan2(d.x) + PI) / TAU * NEB_W as f32 - 0.5;
        let v = (0.5 - d.y.clamp(-1.0, 1.0).asin() / PI) * NEB_H as f32 - 0.5;
        let (x0, y0) = (u.floor(), v.floor());
        let (fx, fy) = (u - x0, v - y0);
        let at = |x: i32, y: i32| {
            let x = x.rem_euclid(NEB_W as i32) as usize;
            let y = y.clamp(0, NEB_H as i32 - 1) as usize;
            self.nebula[y * NEB_W + x]
        };
        let (x, y) = (x0 as i32, y0 as i32);
        let top = at(x, y) * (1.0 - fx) + at(x + 1, y) * fx;
        let bottom = at(x, y + 1) * (1.0 - fx) + at(x + 1, y + 1) * fx;
        top * (1.0 - fy) + bottom * fy
    }

    /// Dibuja las estrellas encima del fondo (solo en píxeles donde no hay ningún objeto).
    pub fn draw_stars(&self, fb: &mut Framebuffer, view_proj: &Mat4, time: f32) {
        let (w, h) = (fb.width as f32, fb.height as f32);
        for s in &self.stars {
            let c = math::transform(view_proj, s.dir, 0.0);
            if c[3] <= 0.0 {
                continue;
            }
            let x = ((c[0] / c[3] + 1.0) * 0.5 * w) as i32;
            let y = ((1.0 - c[1] / c[3]) * 0.5 * h) as i32;
            if x < 0 || y < 0 || x >= fb.width as i32 || y >= fb.height as i32 {
                continue;
            }
            let i = y as usize * fb.width + x as usize;
            if fb.gbuf[i].id == 0 {
                // Las brillantes parpadean un poco.
                let color = if s.big { s.color * (0.8 + 0.2 * (time * 2.3 + s.twinkle).sin()) } else { s.color };
                fb.color[i] += color;
                if s.big {
                    // Las estrellas brillantes ocupan una crucecita, pero sin tapar objetos.
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx >= 0
                            && ny >= 0
                            && nx < fb.width as i32
                            && ny < fb.height as i32
                            && fb.gbuf[ny as usize * fb.width + nx as usize].id == 0
                        {
                            fb.blend(nx, ny, color * 0.6, 0.5);
                        }
                    }
                }
            }
        }
    }
}
