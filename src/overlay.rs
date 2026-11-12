//! Lo que se dibuja encima del cuadro ya sombreado: órbitas punteadas (con prueba de
//! profundidad, así los planetas las tapan), el brillo del sol y el minimapa.
use crate::framebuffer::Framebuffer;
use crate::math::{self, Mat4};
use crate::system::System;
use raylib::prelude::*;
use std::f32::consts::TAU;

/// Línea 3D entre dos puntos ya proyectados (x, y en píxeles, iw = 1/w).
/// Se dibuja punteada y solo donde está más cerca que lo que ya hay en el depth buffer.
fn line(fb: &mut Framebuffer, a: (f32, f32, f32), b: (f32, f32, f32), color: Vector3, alpha: f32, dash: &mut u32) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as i32;
    if steps > 4000 {
        return; // segmento enorme (casi detrás de la cámara): no vale la pena
    }
    for i in 0..=steps {
        *dash += 1;
        if (*dash / 4) % 2 == 1 {
            continue;
        }
        let t = i as f32 / steps as f32;
        let (x, y) = ((a.0 + dx * t) as i32, (a.1 + dy * t) as i32);
        if x < 0 || y < 0 || x >= fb.width as i32 || y >= fb.height as i32 {
            continue;
        }
        let iw = a.2 + (b.2 - a.2) * t;
        if iw > fb.depth[y as usize * fb.width + x as usize] {
            fb.blend(x, y, color, alpha);
        }
    }
}

pub fn orbits(fb: &mut Framebuffer, vp: &Mat4, system: &System, t: f32, highlight: Option<usize>) {
    let (w, h) = (fb.width as f32, fb.height as f32);
    for (i, body) in system.bodies.iter().enumerate().skip(1) {
        let segments = 240;
        let mut prev: Option<(f32, f32, f32)> = None;
        let mut dash = 0;
        let alpha = if highlight == Some(i) { 0.8 } else { 0.35 };
        let center = body.orbit_center(t); // el sol, o el planeta si es una luna
        for k in 0..=segments {
            let a = k as f32 / segments as f32 * TAU;
            let p = center + Vector3::new(a.cos(), 0.0, a.sin()) * body.orbit_radius;
            let c = math::transform(vp, p, 1.0);
            let cur =
                if c[3] > 0.5 { Some(((c[0] / c[3] + 1.0) * 0.5 * w, (1.0 - c[1] / c[3]) * 0.5 * h, 1.0 / c[3])) } else { None };
            if let (Some(a), Some(b)) = (prev, cur) {
                line(fb, a, b, body.color, alpha, &mut dash);
            }
            prev = cur;
        }
    }
}

/// Estela de los motores: puntos que se van apagando (los más viejos primero).
pub fn trail(fb: &mut Framebuffer, vp: &Mat4, points: &[Vector3]) {
    let (w, h) = (fb.width as f32, fb.height as f32);
    let n = points.len().max(1) as f32;
    for (i, p) in points.iter().enumerate() {
        let c = math::transform(vp, *p, 1.0);
        if c[3] <= 0.1 {
            continue;
        }
        let (x, y) = ((c[0] / c[3] + 1.0) * 0.5 * w, (1.0 - c[1] / c[3]) * 0.5 * h);
        let (xi, yi) = (x as i32, y as i32);
        if xi < 0 || yi < 0 || xi >= fb.width as i32 || yi >= fb.height as i32 {
            continue;
        }
        if 1.0 / c[3] < fb.depth[yi as usize * fb.width + xi as usize] {
            continue; // algo lo tapa
        }
        let age = i as f32 / n; // 0 = el más viejo
        let color = Vector3::new(1.0, 0.45 + age * 0.4, 0.15 + age * 0.3);
        fb.blend(xi, yi, color, age * 0.8);
        if age > 0.6 {
            fb.blend(xi + 1, yi, color, age * 0.4);
            fb.blend(xi, yi + 1, color, age * 0.4);
        }
    }
}

/// Halo de atmósfera alrededor de un planeta: luz fuera de la silueta, más fuerte del lado
/// que mira al sol. Para cada píxel del fondo se calcula a qué distancia pasa su rayo de
/// vista del centro del planeta: si pasa un poco por fuera del radio, ahí hay atmósfera.
/// `ray(x, y)` da la dirección de vista de un píxel.
pub fn atmosphere(fb: &mut Framebuffer, ray: &dyn Fn(usize, usize) -> Vector3, center: Vector3, radius: f32, eye: Vector3, sun: Vector3, color: Vector3) {
    let to_c = center - eye;
    let dist = to_c.length();
    let thick = radius * 0.08;
    // Si se ve muy chiquito no vale la pena.
    if radius / dist < 0.01 {
        return;
    }
    let to_sun = (sun - center).normalized();
    let (w, h) = (fb.width, fb.height);
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if fb.gbuf[i].id != 0 {
                continue;
            }
            let d = ray(x, y);
            let along = to_c.dot(d);
            if along <= 0.0 {
                continue;
            }
            let closest = d * along - to_c; // del centro al punto más cercano del rayo
            let miss = closest.length();
            if miss < radius || miss > radius + thick {
                continue;
            }
            let side = (closest.normalized().dot(to_sun) * 0.5 + 0.5).powi(2);
            let a = (1.0 - (miss - radius) / thick).powi(2) * 0.55 * (0.15 + side * 0.85);
            fb.color[i] += color * a;
        }
    }
}

/// Halo alrededor del sol: se suma luz que cae con la distancia al disco.
/// Solo se suma en píxeles que están detrás del sol (o en el fondo).
pub fn sun_glow(fb: &mut Framebuffer, vp: &Mat4, proj: &Mat4, sun: Vector3, radius: f32, eye: Vector3) {
    let c = math::transform(vp, sun, 1.0);
    if c[3] <= 0.0 {
        return;
    }
    let (w, h) = (fb.width as f32, fb.height as f32);
    let (cx, cy) = ((c[0] / c[3] + 1.0) * 0.5 * w, (1.0 - c[1] / c[3]) * 0.5 * h);
    let dist = (sun - eye).length();
    let r = radius * proj[1][1] * h * 0.5 / dist;
    let reach = (r * 3.0).min(w);
    let sun_iw = 1.0 / dist;
    let (x0, x1) = ((cx - reach).max(0.0) as i32, (cx + reach).min(w - 1.0) as i32);
    let (y0, y1) = ((cy - reach).max(0.0) as i32, (cy + reach).min(h - 1.0) as i32);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / r;
            if d <= 0.98 {
                continue;
            }
            let i = y as usize * fb.width + x as usize;
            if fb.depth[i] > sun_iw && fb.gbuf[i].id != 1 {
                continue; // hay algo entre la cámara y el sol
            }
            let g = (-(d - 1.0) * 2.2).exp() * 0.55;
            fb.color[i] += Vector3::new(1.0, 0.55, 0.18) * g;
        }
    }
}

/// Minimapa visto desde arriba (plano eclíptico) en la esquina superior derecha.
pub fn minimap(fb: &mut Framebuffer, system: &System, t: f32, ship: Vector3, forward: Vector3, highlight: Option<usize>) {
    let size = (fb.height as f32 * 0.32) as i32;
    let margin = 8;
    let (ox, oy) = (fb.width as i32 - size - margin, margin);
    let half = size as f32 / 2.0;
    let (cx, cy) = (ox as f32 + half, oy as f32 + half);
    let max_orbit = system.bodies.iter().map(|b| b.orbit_radius).fold(1.0, f32::max) * 1.08;
    let k = (half - 4.0) / max_orbit;
    let to_px = |p: Vector3| (cx + p.x * k, cy + p.z * k);

    // Fondo oscuro con borde.
    for y in oy..oy + size {
        for x in ox..ox + size {
            let border = x == ox || y == oy || x == ox + size - 1 || y == oy + size - 1;
            if border {
                fb.blend(x, y, Vector3::new(0.5, 0.6, 0.8), 0.6);
            } else {
                fb.blend(x, y, Vector3::new(0.0, 0.0, 0.02), 0.65);
            }
        }
    }
    let disc = |fb: &mut Framebuffer, px: f32, py: f32, r: f32, c: Vector3| {
        let ri = r.ceil() as i32;
        for y in -ri..=ri {
            for x in -ri..=ri {
                if (x * x + y * y) as f32 <= r * r {
                    fb.blend(px as i32 + x, py as i32 + y, c, 1.0);
                }
            }
        }
    };
    for (i, b) in system.bodies.iter().enumerate() {
        if i > 0 {
            let steps = 90;
            for s in 0..steps {
                let a = s as f32 / steps as f32 * TAU;
                let (x, y) = to_px(b.orbit_center(t) + Vector3::new(a.cos(), 0.0, a.sin()) * b.orbit_radius);
                let alpha = if highlight == Some(i) { 0.7 } else { 0.25 };
                fb.blend(x as i32, y as i32, b.color, alpha);
            }
        }
        let (x, y) = to_px(b.center(t));
        disc(fb, x, y, (b.radius() * k * 2.0).clamp(1.5, 5.0), b.color);
    }
    // La nave: un punto blanco con una rayita hacia donde apunta.
    let (sx, sy) = to_px(ship);
    let f = Vector3::new(forward.x, 0.0, forward.z);
    let f = if f.length() > 1e-3 { f.normalized() } else { Vector3::new(0.0, 0.0, 1.0) };
    for s in 0..7 {
        fb.blend((sx + f.x * s as f32) as i32, (sy + f.z * s as f32) as i32, Vector3::one(), 0.9);
    }
    disc(fb, sx, sy, 1.6, Vector3::one());
}
