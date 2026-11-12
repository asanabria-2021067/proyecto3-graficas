//! Ignis: planeta de lava. La corteza está partida en placas (ruido celular) y por las
//! grietas corre lava que brilla sola. En las partes bajas hay mares de lava que fluyen:
//! el ruido se deforma con otro ruido que cambia con el tiempo (shader animado).
//! `intensity` = qué tan caliente está (grietas más anchas y lava más brillante).
use super::noise_helpers::{cellular, fbm, fbm4, sample4};
use super::{mix, ramp, rgb, smoothstep, surface, Globals, ShadeObject};
use crate::framebuffer::GSample;
use raylib::prelude::*;

fn lava_color(heat: f32) -> Vector3 {
    ramp(
        &[
            (0.0, rgb(0.25, 0.02, 0.0)),
            (0.35, rgb(0.85, 0.12, 0.01)),
            (0.65, rgb(1.0, 0.45, 0.03)),
            (0.9, rgb(1.0, 0.85, 0.35)),
        ],
        heat,
    )
}

pub fn lava(s: &GSample, o: &ShadeObject, g: &Globals) -> Vector3 {
    let sf = surface(s, o, g);
    let (p, n, l, v) = (sf.p, sf.n, sf.l, sf.v);
    let nz = g.noise;
    let t = g.time;
    let hot = o.intensity;

    // ---------- 1. Placas de corteza ----------
    let warp = Vector3::new(fbm(nz, p * 2.0, 2), fbm(nz, p * 2.0 + Vector3::new(4.0, 1.0, 0.0), 2), 0.0);
    let (f1, f2) = cellular(p * 5.0 + warp * 0.6);
    let edge = f2 - f1; // ~0 en la frontera entre placas
    let width = 0.05 + hot * 0.08 + fbm(nz, p * 6.0, 2) * 0.03;
    let crack = 1.0 - smoothstep(0.0, width, edge);

    // ---------- 2. Mares de lava en las zonas bajas ----------
    let low = fbm(nz, p * 1.4 + Vector3::new(9.0, 3.0, 1.0), 3);
    let sea = smoothstep(-0.05, -0.18, low - hot * 0.08);

    // Flujo: el patrón se deforma con un ruido 4D (el tiempo es la cuarta coordenada),
    // así la lava parece moverse y burbujear sin deslizarse en bloque.
    let flow_warp = Vector3::new(
        fbm4(nz, p * 2.5, t * 0.12, 2),
        fbm4(nz, p * 2.5 + Vector3::new(3.3, 7.1, 1.2), t * 0.12, 2),
        fbm4(nz, p * 2.5 + Vector3::new(8.2, 0.4, 5.5), t * 0.12, 2),
    );
    let flow = fbm(nz, p * 6.0 + flow_warp * 1.6, 3) * 0.5 + 0.5;
    // Pulso lento: la lava de las grietas "respira".
    let pulse = sample4(nz, p * 3.0, t * 0.4) * 0.5 + 0.5;

    let molten = crack.max(sea);
    let heat = (flow * 0.7 + pulse * 0.3) * (0.6 + hot * 0.5) + crack * 0.25;
    let glow = lava_color(heat.clamp(0.0, 1.0)) * (0.9 + hot * 0.9);

    // ---------- 3. Corteza: basalto oscuro con relieve ----------
    let rock_var = fbm(nz, p * 9.0, 3) * 0.5 + 0.5;
    let mut crust = mix(rgb(0.07, 0.05, 0.05), rgb(0.20, 0.15, 0.13), rock_var);
    // La orilla de cada placa está más caliente: un poco rojiza.
    crust = mix(crust, rgb(0.35, 0.06, 0.02), (1.0 - smoothstep(0.0, width * 3.0, edge)) * 0.6);

    let shading_n = if g.normal_mapping { g.maps.rock.triplanar(p, n, 4.0, 0.9 * (1.0 - molten)) } else { n };
    let diffuse = shading_n.dot(l).max(0.0) * smoothstep(-0.05, 0.1, n.dot(l));
    let half = (l + v).normalized();
    let spec = shading_n.dot(half).max(0.0).powf(24.0) * 0.15;
    let crust_lit = crust * (0.03 + diffuse) + rgb(1.0, 0.9, 0.8) * spec * diffuse.min(1.0);

    // La lava brilla sola (no depende del sol). La corteza ilumina un poco su alrededor.
    let mut color = mix(crust_lit, glow, molten);
    color = color + rgb(0.6, 0.1, 0.0) * ((1.0 - molten) * crack.max(sea * 0.5) * 0.2);

    // Humo de la atmósfera: halo rojizo en el borde, se ve también de noche.
    let rim = (1.0 - n.dot(v).max(0.0)).powf(2.5);
    color + rgb(0.9, 0.25, 0.05) * (rim * (0.25 + hot * 0.3))
}
