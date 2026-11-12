//! Solmira: el sol del sistema (viene del lab 6). Convección animada con ruido 4D,
//! granulación, manchas solares y oscurecimiento del borde.
//! No depende de la luz: la estrella ES la fuente de luz.
use super::noise_helpers::{fbm4, sample4};
use super::{ramp, rgb, smoothstep, surface, Globals, ShadeObject};
use crate::framebuffer::GSample;
use raylib::prelude::*;

pub fn star(s: &GSample, o: &ShadeObject, g: &Globals) -> Vector3 {
    let sf = surface(s, o, g);
    let p = sf.p;
    let t = g.time;
    let nz = g.noise;

    // 1) Turbulencia grande: fBm 4D con "domain warping". El tiempo es la 4ta dimensión,
    //    por eso el plasma hierve en su lugar en vez de solo deslizarse.
    let warp = Vector3::new(
        fbm4(nz, p * 2.0, t * 0.15, 2),
        fbm4(nz, p * 2.0 + Vector3::new(5.2, 1.3, 2.8), t * 0.15, 2),
        fbm4(nz, p * 2.0 + Vector3::new(1.7, 9.2, 4.1), t * 0.15, 2),
    );
    let heat = fbm4(nz, p * 3.0 + warp * 1.4, t * 0.3, 4) * 0.5 + 0.5;

    // 2) Granulación: donde el ruido cruza por 0 quedan canales oscuros entre celdas.
    let gran = sample4(nz, p * 20.0 + warp * 0.6, t * 0.9);
    let lanes = smoothstep(0.0, 0.12, gran.abs());

    // 3) Manchas solares: solo en dos franjas de latitud. `intensity` = actividad.
    let lat = p.y.abs();
    let belt = smoothstep(0.08, 0.22, lat) * (1.0 - smoothstep(0.45, 0.62, lat));
    let spot_n = sample4(nz, p * 2.6 + Vector3::new(11.0, 0.0, 3.0), t * 0.05) * 0.5 + 0.5;
    let threshold = 0.78 - o.intensity * 0.18;
    let penumbra = smoothstep(threshold, threshold + 0.05, spot_n) * belt;
    let umbra = smoothstep(threshold + 0.06, threshold + 0.1, spot_n) * belt;

    let mut temp = 0.12 + heat * 0.85 + lanes * 0.10;
    temp *= 1.0 - penumbra * 0.35 - umbra * 0.45;

    // 4) Limb darkening: en el borde se ven capas más frías.
    let mu = sf.n.dot(sf.v).max(0.0);
    temp *= 0.55 + 0.45 * mu.sqrt();

    let color = ramp(
        &[
            (0.0, rgb(0.20, 0.01, 0.0)),
            (0.30, rgb(0.70, 0.12, 0.02)),
            (0.55, rgb(1.0, 0.45, 0.06)),
            (0.75, rgb(1.0, 0.78, 0.30)),
            (0.95, rgb(1.0, 0.97, 0.85)),
        ],
        temp,
    );

    // 5) Corona: brillo naranja que crece hacia el borde.
    let rim = (1.0 - mu).powf(3.0);
    color * 1.15 + rgb(1.0, 0.35, 0.05) * rim * 0.9
}
