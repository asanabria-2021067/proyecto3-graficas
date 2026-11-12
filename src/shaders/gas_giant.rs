//! Orvane: gigante gaseoso (viene del lab 6). Bandas que fluyen como líquido y una
//! tormenta que tuerce las bandas a su alrededor. `intensity` = fuerza de la tormenta.
use super::noise_helpers::{fbm, fbm4};
use super::{mix, ramp, rgb, smoothstep, surface, Globals, ShadeObject};
use crate::framebuffer::GSample;
use crate::math::{rotate_axis, rotate_y};
use noise::OpenSimplex;
use raylib::prelude::*;
use std::f32::consts::PI;

const STORM_RADIUS: f32 = 0.34;
const BAND_COUNT: f32 = 9.0;
const TURBULENCE: f32 = 0.14;
const FLOW_SPEED: f32 = 0.08;

pub fn gas_giant(s: &GSample, o: &ShadeObject, g: &Globals) -> Vector3 {
    let sf = surface(s, o, g);
    let (p, n, l, v) = (sf.p, sf.n, sf.l, sf.v);
    let nz = g.noise;
    let t = g.time;
    let storm_strength = o.intensity * 10.0;

    // ---------- 1. Tormenta: torsión localizada ----------
    // Cada punto cercano al centro se gira alrededor del eje que pasa por el centro.
    // Como las bandas se calculan después con el punto girado, las bandas se curvan.
    let (lat0, lon0) = (-0.38f32, 0.6f32);
    let c = Vector3::new(lat0.cos() * lon0.sin(), lat0.sin(), lat0.cos() * lon0.cos());
    let d = p.dot(c).clamp(-1.0, 1.0).acos();
    let falloff = (1.0 - d / STORM_RADIUS).clamp(0.0, 1.0);
    let ps = rotate_axis(p, c, storm_strength * falloff * falloff);

    // ---------- 2. Flujo de las bandas (flow map) ----------
    // Dos copias del patrón avanzan desfasadas medio ciclo y se mezclan para que las
    // bandas no se estiren para siempre.
    let jet = (ps.y * BAND_COUNT * 0.5 * PI).sin();
    let cycle = t * FLOW_SPEED;
    let phase0 = cycle.fract();
    let phase1 = (cycle + 0.5).fract();
    let (val0, r0) = bands(nz, ps, jet * (phase0 - 0.5) * 0.6, t);
    let (val1, r1) = bands(nz, ps, jet * (phase1 - 0.5) * 0.6, t);
    let blend = ((phase0 - 0.5) * 2.0).abs();
    let val = val0 + (val1 - val0) * blend;
    let r = r0 + (r1 - r0) * blend;

    let mut color = ramp(
        &[
            (0.0, rgb(0.32, 0.18, 0.24)),
            (0.25, rgb(0.62, 0.30, 0.22)),
            (0.45, rgb(0.84, 0.60, 0.40)),
            (0.65, rgb(0.95, 0.87, 0.70)),
            (0.82, rgb(0.62, 0.55, 0.68)),
            (1.0, rgb(0.92, 0.88, 0.82)),
        ],
        val,
    );

    // ---------- 3. Ojo y collar de la tormenta ----------
    let k = (storm_strength / 5.0).min(1.0);
    let core = smoothstep(STORM_RADIUS * 0.6, STORM_RADIUS * 0.2, d);
    let collar = smoothstep(STORM_RADIUS * 0.75, STORM_RADIUS * 0.58, d)
        * (1.0 - smoothstep(STORM_RADIUS * 0.58, STORM_RADIUS * 0.45, d));
    let storm_col = mix(rgb(0.55, 0.12, 0.06), rgb(0.95, 0.45, 0.18), r * 0.5 + 0.5);
    color = mix(color, rgb(0.97, 0.90, 0.78), collar * 0.55 * k);
    color = mix(color, storm_col, core * k);

    // Polos más oscuros y azulados.
    color = mix(color, rgb(0.28, 0.30, 0.42), smoothstep(0.72, 0.95, p.y.abs()) * 0.7);

    // ---------- 4. Luz difusa "envuelta" (como una atmósfera gruesa) ----------
    let wrap = ((n.dot(l) + 0.2) / 1.2).max(0.0);
    let rim = (1.0 - n.dot(v).max(0.0)).powf(2.5);
    color * (0.02 + wrap * 1.05) + rgb(0.9, 0.7, 0.5) * (rim * wrap * 0.25)
}

/// Patrón de bandas para una posición ya torcida por la tormenta.
fn bands(nz: &OpenSimplex, ps: Vector3, shift: f32, t: f32) -> (f32, f32) {
    let pf = rotate_y(ps, shift);
    let q = Vector3::new(
        fbm4(nz, pf * 1.2 + Vector3::new(1.0, 0.0, 4.0), t * 0.04, 2),
        fbm4(nz, pf * 1.2 + Vector3::new(6.0, 2.0, 0.0), t * 0.04, 2),
        0.0,
    );
    let r = fbm4(nz, pf * 2.0 + q * 2.6, t * 0.07, 3);
    let y = ps.y + r * TURBULENCE + q.x * TURBULENCE * 0.5;
    let b1 = (y * BAND_COUNT * PI).sin() * 0.5 + 0.5;
    let b2 = (y * BAND_COUNT * 2.3 * PI + q.y * 3.0).sin() * 0.5 + 0.5;
    let streaks = fbm(nz, Vector3::new(pf.x * 3.0, pf.y * 38.0, pf.z * 3.0) + q, 3) * 0.5 + 0.5;
    (b1 * 0.62 + b2 * 0.25 + streaks * 0.13, r)
}
