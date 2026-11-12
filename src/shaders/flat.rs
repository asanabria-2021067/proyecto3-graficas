//! Shaders de los modelos .obj: los objetos de la superficie y la nave.
//! Usan el color del material (Kd) y la luz del sol como luz puntual.
use super::{Globals, ShadeObject, rgb, smoothstep};
use crate::framebuffer::GSample;
use raylib::prelude::*;

/// Objetos de la superficie (árboles, rocas, casas, cristales, obelisco).
/// Los materiales "Brillo" (cristales y ventanas) laten despacio y se ven de noche.
pub fn flat(s: &GSample, o: &ShadeObject, g: &Globals) -> Vector3 {
    let n = s.normal.normalized();
    let l = (g.sun - s.world).normalized();
    // ¿Es de día en este punto del planeta? (la normal del suelo es la dirección desde el centro)
    let ground = (s.world - o.center).normalized();
    let day = smoothstep(-0.12, 0.2, ground.dot(l));
    // Lambert "envuelto": las caras de espaldas al sol no quedan negras, como si el cielo
    // y el suelo rebotaran un poco de luz.
    let wrap = (n.dot(l) * 0.5 + 0.5).powi(2);
    let lit = s.color * (0.03 + wrap * day * 1.1);
    let pulse = 0.75 + 0.25 * (g.time * 2.0 + s.world.x * 0.7 + s.world.z * 0.5).sin();
    lit + s.color * (s.emit * pulse * 0.9)
}

/// La nave: difusa + especular (Blinn-Phong) y los motores brillan con un parpadeo.
/// `intensity` = empuje actual, así el fuego crece al acelerar.
pub fn ship(s: &GSample, o: &ShadeObject, g: &Globals) -> Vector3 {
    let n = s.normal.normalized();
    let l = (g.sun - s.world).normalized();
    let v = (g.camera - s.world).normalized();
    let diffuse = n.dot(l).max(0.0);
    let half = (l + v).normalized();
    let spec = n.dot(half).max(0.0).powf(40.0) * 0.6;
    // Luz de relleno desde la cámara para que la nave se distinga del lado de noche.
    let fill = n.dot(v).max(0.0) * 0.28;
    let rim = (1.0 - n.dot(v).max(0.0)).powf(3.0) * 0.25;
    let base = s.color * (0.05 + diffuse + fill) + rgb(1.0, 0.97, 0.9) * spec * diffuse + rgb(0.4, 0.6, 1.0) * rim;
    let flicker = 0.85 + 0.15 * (g.time * 37.0).sin() * (g.time * 23.0).cos();
    let engine = rgb(1.0, 0.55, 0.15) * (s.emit * (0.6 + o.intensity * 1.6) * flicker);
    base * (1.0 - s.emit) + engine
}
