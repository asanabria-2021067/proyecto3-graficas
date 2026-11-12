//! La serpiente de roca de Tessara: piedra oscura con vetas que brillan. Un pulso de luz
//! recorre el cuerpo desde la cabeza hasta la cola (shader animado).
//! `intensity` = posición del segmento en el cuerpo (0 = cabeza, 1 = cola); los ojos usan -1.
//! `center` = centro del planeta, para saber si en ese punto es de día.
use super::noise_helpers::{cellular, fbm};
use super::{mix, ramp, rgb, smoothstep, Globals, ShadeObject};
use crate::framebuffer::GSample;
use raylib::prelude::*;

pub fn serpent(s: &GSample, o: &ShadeObject, g: &Globals) -> Vector3 {
    let t = g.time;
    if o.intensity < 0.0 {
        // Ojos: brillan solos y parpadean de vez en cuando.
        let blink = smoothstep(0.0, 0.08, ((t * 0.5).fract() - 0.02).abs());
        return rgb(1.0, 0.55, 0.15) * (1.6 * blink + 0.2);
    }
    let param = o.intensity;
    let n = s.normal.normalized();
    let l = (g.sun - s.world).normalized();
    let v = (g.camera - s.world).normalized();
    let day = smoothstep(-0.15, 0.2, (s.world - o.center).normalized().dot(l));

    // Piedra: dos tonos mezclados con ruido (cada segmento distinto gracias a `param`).
    let q = s.obj * 3.0 + Vector3::new(param * 13.0, param * 7.0, 0.0);
    let stone = mix(rgb(0.10, 0.10, 0.13), rgb(0.30, 0.27, 0.30), fbm(g.noise, q, 3) * 0.5 + 0.5);

    // Vetas: fronteras de celdas de Voronoi.
    let (f1, f2) = cellular(q * 1.4);
    let vein = 1.0 - smoothstep(0.0, 0.09, f2 - f1);

    // Pulso: una onda que baja por el cuerpo.
    let wave = ((t * 2.5 - param * 14.0).sin() * 0.5 + 0.5).powf(4.0);
    let glow = ramp(&[(0.0, rgb(0.05, 0.55, 0.65)), (0.5, rgb(0.35, 0.35, 0.95)), (1.0, rgb(0.85, 0.25, 0.75))], param);

    let wrap = (n.dot(l) * 0.5 + 0.5).powi(2);
    let half = (l + v).normalized();
    let spec = n.dot(half).max(0.0).powf(30.0) * 0.25;
    let lit = stone * (0.04 + wrap * day * 1.1) + rgb(1.0, 0.95, 0.9) * spec * day;
    lit + glow * (vein * (0.35 + wave * 1.8) + wave * 0.12)
}
