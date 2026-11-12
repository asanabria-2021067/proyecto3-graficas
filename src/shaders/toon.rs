//! Mirra: planeta miniatura con estilo de caricatura. La luz no es continua sino en
//! tres bandas (cel shading), el borde lleva una línea oscura y la superficie tiene un
//! camino de baldosas que da vueltas en espiral de un polo al otro.
//! `intensity` = cantidad de florecitas en el pasto.
use super::noise_helpers::{cellular, fbm, hash3};
use super::{mix, rgb, smoothstep, surface, Globals, ShadeObject};
use crate::framebuffer::GSample;
use raylib::prelude::*;
use std::f32::consts::{PI, TAU};

const TURNS: f32 = 2.0; // vueltas que da el camino entre los polos
const PATH_WIDTH: f32 = 0.11;

/// Coordenadas del camino en espiral: (distancia al centro del camino, avance a lo largo).
/// La espiral es lon = TURNS * 2 * lat: la diferencia con la longitud del punto, medida
/// sobre el paralelo (por eso el cos(lat)), es la distancia al camino.
pub fn path_coords(p: Vector3) -> (f32, f32) {
    let lat = p.y.clamp(-1.0, 1.0).asin();
    let lon = p.z.atan2(p.x);
    let diff = (lon - lat * TURNS * 2.0).rem_euclid(TAU);
    let diff = if diff > PI { diff - TAU } else { diff };
    (diff.abs() * lat.cos(), lat)
}

pub fn on_path(p: Vector3) -> bool {
    path_coords(p).0 < PATH_WIDTH * 1.6
}

pub fn toon(s: &GSample, o: &ShadeObject, g: &Globals) -> Vector3 {
    let sf = surface(s, o, g);
    let (p, n, l, v) = (sf.p, sf.n, sf.l, sf.v);
    let nz = g.noise;

    // ---------- Pasto con florecitas ----------
    let tone = fbm(nz, p * 3.0, 2) * 0.5 + 0.5;
    let mut albedo = mix(rgb(0.16, 0.52, 0.50), rgb(0.30, 0.68, 0.58), smoothstep(0.35, 0.65, tone));
    let (f1, _) = cellular(p * 22.0);
    let cell = p * 22.0;
    let pick = hash3(cell.x.floor() as i32, cell.y.floor() as i32, cell.z.floor() as i32);
    if f1 < 0.12 && pick < o.intensity * 0.6 {
        albedo = if pick < o.intensity * 0.3 { rgb(0.98, 0.62, 0.72) } else { rgb(1.0, 0.88, 0.45) };
    }

    // ---------- Camino de baldosas ----------
    let (d, along) = path_coords(p);
    let path = 1.0 - smoothstep(PATH_WIDTH - 0.01, PATH_WIDTH + 0.01, d);
    let border = smoothstep(PATH_WIDTH - 0.03, PATH_WIDTH - 0.01, d) * path;
    // Baldosas: filas a lo largo del camino y una junta al centro, desfasada en cada fila.
    let row = along * 40.0;
    let offset = if (row.floor() as i32) % 2 == 0 { 0.0 } else { 0.5 };
    let col = d / PATH_WIDTH * 1.5 + offset;
    let edge = |x: f32| 1.0 - smoothstep(0.0, 0.1, x.fract().min(1.0 - x.fract()));
    let joint = edge(row).max(edge(col));
    let tile = mix(rgb(0.86, 0.50, 0.36), rgb(0.62, 0.30, 0.24), joint * 0.7);
    albedo = mix(albedo, tile, path);
    albedo = mix(albedo, rgb(0.95, 0.88, 0.75), border);

    // ---------- Cel shading: tres tonos ----------
    let ndl = n.dot(l);
    let band = smoothstep(-0.02, 0.02, ndl) * 0.45 + smoothstep(0.38, 0.42, ndl) * 0.55;
    let shadow_tint = rgb(0.30, 0.25, 0.55); // la sombra es morada, no negra
    let mut color = mix(albedo * shadow_tint * 0.35, albedo, band);

    // Brillo de caricatura: una mancha blanca recortada.
    let half = (l + v).normalized();
    color = color + rgb(1.0, 1.0, 1.0) * smoothstep(0.993, 0.995, n.dot(half)) * 0.18 * (1.0 - path);

    // Contorno oscuro en la silueta + luz de borde cálida.
    let facing = n.dot(v);
    let rim = smoothstep(0.35, 0.2, facing) * smoothstep(-0.1, 0.3, ndl);
    color = mix(color, rgb(1.0, 0.85, 0.6), rim * 0.35);
    mix(color, rgb(0.05, 0.03, 0.10), smoothstep(0.16, 0.08, facing))
}
