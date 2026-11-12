//! Anillo de Orvane. La malla es un disco plano; los huecos entre anillos se recortan en
//! el rasterizado (prueba de transparencia) y el color sale de bandas según el radio.
//! También recibe la sombra del planeta.
use super::{mix, rgb, smoothstep, Globals, ShadeObject};
use crate::framebuffer::GSample;
use raylib::prelude::*;

pub const INNER: f32 = 1.35;
pub const OUTER: f32 = 2.3;

fn radius(obj: Vector3) -> f32 {
    (obj.x * obj.x + obj.z * obj.z).sqrt()
}

/// Densidad del anillo según la distancia al centro (0 = hueco).
fn density(r: f32) -> f32 {
    let x = (r - INNER) / (OUTER - INNER);
    let bands = (x * 61.0).sin() * 0.25 + (x * 23.0 + 1.3).sin() * 0.35 + (x * 7.0).sin() * 0.2 + 0.55;
    let gap = smoothstep(0.012, 0.03, (x - 0.47).abs()); // la división grande
    let edges = smoothstep(0.0, 0.04, x) * smoothstep(1.0, 0.92, x);
    bands.clamp(0.0, 1.0) * gap * edges
}

/// La usa el rasterizador: si devuelve false el píxel no se dibuja.
pub fn alpha(obj: Vector3) -> bool {
    density(radius(obj)) > 0.18
}

pub fn ring(s: &GSample, o: &ShadeObject, g: &Globals) -> Vector3 {
    let r = radius(s.obj);
    let dens = density(r);
    let x = (r - INNER) / (OUTER - INNER);
    let base = mix(rgb(0.55, 0.42, 0.34), rgb(0.92, 0.84, 0.70), (x * 13.0).sin() * 0.5 + 0.5);
    let albedo = mix(base * 0.6, base, dens);

    // Sombra del planeta: ¿el rayo de este punto hacia el sol choca con la esfera?
    let to_sun = (g.sun - s.world).normalized();
    let oc = s.world - o.center;
    let b = oc.dot(to_sun);
    let c = oc.dot(oc) - o.radius * o.radius;
    let shadow = if b < 0.0 && b * b - c > 0.0 { 0.08 } else { 1.0 };

    // El anillo es delgado: deja pasar luz, así que se ilumina por los dos lados.
    let ndl = s.normal.normalized().dot(to_sun).abs();
    albedo * (0.04 + (0.35 + ndl * 0.65) * shadow)
}
