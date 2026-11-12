//! Glacia: planeta de hielo. Campos de nieve, grietas grandes (ruido celular), mares
//! congelados que brillan, relieve con el mapa de normales "agrietado" y auroras
//! animadas en los polos del lado de noche. `intensity` = fuerza de la aurora.
use super::noise_helpers::{cellular, fbm, fbm4};
use super::{mix, rgb, smoothstep, surface, Globals, ShadeObject};
use crate::framebuffer::GSample;
use raylib::prelude::*;

pub fn ice(s: &GSample, o: &ShadeObject, g: &Globals) -> Vector3 {
    let sf = surface(s, o, g);
    let (p, n, l, v) = (sf.p, sf.n, sf.l, sf.v);
    let nz = g.noise;
    let t = g.time;

    // ---------- 1. Color de la superficie ----------
    let drift = fbm(nz, p * 3.0, 3) * 0.5 + 0.5;
    let mut albedo = mix(rgb(0.70, 0.80, 0.92), rgb(0.96, 0.98, 1.0), drift);

    // Mares congelados: zonas lisas de hielo azul.
    let sea_n = fbm(nz, p * 1.5 + Vector3::new(2.0, 5.0, 1.0), 3);
    let frozen_sea = smoothstep(-0.12, -0.22, sea_n);
    albedo = mix(albedo, rgb(0.30, 0.62, 0.80), frozen_sea * 0.85);

    // Grietas grandes: frontera entre celdas de Voronoi, más oscuras y azules.
    let (f1, f2) = cellular(p * 4.0 + Vector3::new(fbm(nz, p * 5.0, 2) * 0.4, 0.0, 0.0));
    let crevasse = 1.0 - smoothstep(0.0, 0.06, f2 - f1);
    albedo = mix(albedo, rgb(0.10, 0.30, 0.55), crevasse * (1.0 - frozen_sea) * 0.8);

    // ---------- 2. Relieve: mapa de normales de grietas ----------
    let shading_n = if g.normal_mapping {
        g.maps.cracks.triplanar(p, n, 2.5, 0.75 * (1.0 - frozen_sea * 0.8))
    } else {
        n
    };

    // ---------- 3. Luz ----------
    let ndl = shading_n.dot(l);
    let terminator = smoothstep(-0.05, 0.1, n.dot(l));
    let diffuse = ndl.max(0.0) * terminator;
    // "Subsuperficie" barata: cerca de la línea día/noche el hielo se ve azul por dentro.
    let sss = smoothstep(-0.3, 0.1, n.dot(l)) * (1.0 - smoothstep(0.1, 0.5, n.dot(l)));
    let mut color = albedo * (0.03 + diffuse) + rgb(0.10, 0.35, 0.60) * sss * 0.35;

    // Brillo especular: muy concentrado en los mares congelados, más suave en la nieve.
    let half = (l + v).normalized();
    let ndh = shading_n.dot(half).max(0.0);
    let spec = ndh.powf(120.0) * 1.4 * frozen_sea + ndh.powf(20.0) * 0.18 * (1.0 - frozen_sea);
    color = color + rgb(0.9, 0.97, 1.0) * spec * terminator;

    // ---------- 4. Aurora (animada) ----------
    // Cortinas verdes cerca de los polos que ondulan con ruido 4D. Solo se ven de noche.
    let lat = p.y.abs();
    let lon = p.z.atan2(p.x);
    let band = smoothstep(0.62, 0.72, lat) * (1.0 - smoothstep(0.80, 0.9, lat));
    let wave = fbm4(nz, Vector3::new(lon.cos() * 2.0, p.y * 6.0, lon.sin() * 2.0), t * 0.35, 2);
    let curtains = ((lon * 9.0 + wave * 6.0 + t * 0.6).sin() * 0.5 + 0.5).powf(3.0);
    let night = 1.0 - smoothstep(-0.2, 0.15, n.dot(l));
    let aurora = band * curtains * night * o.intensity * 1.6;
    color = color + mix(rgb(0.1, 0.9, 0.5), rgb(0.5, 0.3, 1.0), smoothstep(0.7, 0.86, lat)) * aurora;

    // Atmósfera fría en el borde.
    let rim = (1.0 - n.dot(v).max(0.0)).powf(3.0);
    color + rgb(0.45, 0.75, 1.0) * (rim * smoothstep(-0.3, 0.5, n.dot(l)) * 0.6)
}
