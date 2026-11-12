//! Fondo del espacio: una nebulosa tenue y estrellas. Las dos dependen solo de hacia dónde
//! mira la cámara (están "en el infinito"), así que al girar la nave el cielo gira con ella.
use crate::framebuffer::Framebuffer;
use crate::math::{self, Mat4};
use crate::rng::Rng;
use crate::shaders::noise_helpers::fbm;
use crate::shaders::{mix, rgb, smoothstep};
use noise::OpenSimplex;
use raylib::prelude::*;
use rayon::prelude::*;
use std::f32::consts::{PI, TAU};

const NEB_W: usize = 512;
const NEB_H: usize = 256;

struct Star {
    dir: Vector3,
    color: Vector3,
    big: bool,
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

impl Background {
    pub fn new(noise: &OpenSimplex) -> Self {
        // La nebulosa se calcula una vez: fBm sobre la dirección, con dos colores.
        let nebula = (0..NEB_W * NEB_H)
            .into_par_iter()
            .map(|i| {
                let d = dir_from_uv((i % NEB_W) as f32 / NEB_W as f32, (i / NEB_W) as f32 / NEB_H as f32);
                let a = fbm(noise, d * 1.5 + Vector3::new(20.0, 0.0, 0.0), 5) * 0.5 + 0.5;
                let b = fbm(noise, d * 2.5 + Vector3::new(0.0, 30.0, 0.0), 4) * 0.5 + 0.5;
                // Una franja más densa cerca del plano, como la Vía Láctea vista de lado.
                let band = 1.0 - smoothstep(0.0, 0.45, (d.y + 0.25 * (d.x * 2.0).sin()).abs());
                let cloud = smoothstep(0.45, 0.8, a) * (0.4 + band * 0.6);
                let tint = mix(rgb(0.10, 0.03, 0.16), rgb(0.02, 0.07, 0.16), b);
                rgb(0.004, 0.005, 0.012) + tint * cloud * 0.55 + rgb(0.05, 0.04, 0.06) * band * smoothstep(0.5, 0.9, b) * 0.3
            })
            .collect();

        let mut rng = Rng::new(2026);
        let stars = (0..2600)
            .map(|_| {
                let b = rng.f32().powf(6.0); // casi todas tenues, pocas brillantes
                let warm = rng.f32();
                let color = mix(rgb(0.7, 0.8, 1.0), rgb(1.0, 0.85, 0.65), warm) * (0.25 + b * 0.9);
                Star { dir: rng.on_sphere(), color, big: b > 0.55 }
            })
            .collect();
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
    pub fn draw_stars(&self, fb: &mut Framebuffer, view_proj: &Mat4) {
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
                fb.color[i] += s.color;
                if s.big {
                    // Las estrellas brillantes ocupan una crucecita, pero sin tapar objetos.
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx >= 0 && ny >= 0 && nx < fb.width as i32 && ny < fb.height as i32 && fb.gbuf[ny as usize * fb.width + nx as usize].id == 0 {
                            fb.blend(nx, ny, s.color * 0.6, 0.5);
                        }
                    }
                }
            }
        }
    }
}
