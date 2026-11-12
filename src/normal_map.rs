//! Mapeo de normales.
//!
//! Al arrancar se generan dos texturas de normales (no se cargan imágenes): se calcula un
//! campo de altura que se repite sin costuras y la normal de cada texel sale de sus
//! diferencias con los vecinos. La normal se guarda en espacio tangente: z apunta hacia
//! afuera de la superficie y x, y dicen hacia dónde se inclina.
//!
//! En el shader se lee la textura con proyección triplanar (tres proyecciones, una por eje)
//! y cada lectura se pasa al espacio del objeto con su propia base tangente (T, B, N).
use crate::shaders::noise_helpers::hash3;
use crate::shaders::smoothstep;
use noise::{NoiseFn, OpenSimplex};
use raylib::prelude::*;
use rayon::prelude::*;
use std::f32::consts::TAU;

pub struct NormalMap {
    size: usize,
    data: Vec<Vector3>,
}

impl NormalMap {
    /// `height(u, v)` con u, v en [0, 1) debe repetirse en los bordes.
    pub fn from_height(size: usize, strength: f32, height: impl Fn(f32, f32) -> f32 + Sync) -> Self {
        let heights: Vec<f32> = (0..size * size)
            .into_par_iter()
            .map(|i| height((i % size) as f32 / size as f32, (i / size) as f32 / size as f32))
            .collect();
        let h = |x: usize, y: usize| heights[(y % size) * size + (x % size)];
        let mut data = Vec::with_capacity(size * size);
        for y in 0..size {
            for x in 0..size {
                // Diferencias centrales (con vuelta en los bordes).
                let dx = h(x + 1, y) - h(x + size - 1, y);
                let dy = h(x, y + 1) - h(x, y + size - 1);
                data.push(Vector3::new(-dx * strength, -dy * strength, 1.0).normalized());
            }
        }
        Self { size, data }
    }

    /// Lectura bilineal con repetición.
    pub fn sample(&self, u: f32, v: f32) -> Vector3 {
        let s = self.size as f32;
        let (x, y) = (u.rem_euclid(1.0) * s - 0.5, v.rem_euclid(1.0) * s - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let n = self.size as i32;
        let at = |xi: i32, yi: i32| self.data[(yi.rem_euclid(n) * n + xi.rem_euclid(n)) as usize];
        let (xi, yi) = (x0 as i32, y0 as i32);
        let top = at(xi, yi) * (1.0 - fx) + at(xi + 1, yi) * fx;
        let bottom = at(xi, yi + 1) * (1.0 - fx) + at(xi + 1, yi + 1) * fx;
        top * (1.0 - fy) + bottom * fy
    }

    /// Aplica el mapa a la normal `n` de un punto `p` (los dos en espacio del objeto).
    /// `tiling` = cuántas veces se repite la textura por unidad, `strength` = qué tanto inclina.
    pub fn triplanar(&self, p: Vector3, n: Vector3, tiling: f32, strength: f32) -> Vector3 {
        // Pesos: cada proyección cuenta más donde la superficie mira hacia su eje.
        let mut w = Vector3::new(n.x.abs().powi(4), n.y.abs().powi(4), n.z.abs().powi(4));
        w = w * (1.0 / (w.x + w.y + w.z + 1e-6));
        let q = p * tiling;
        // Proyección sobre X: la textura se pone en el plano ZY.
        let tx = self.sample(q.z, q.y);
        // Sobre Y: plano XZ. Sobre Z: plano XY.
        let ty = self.sample(q.x, q.z);
        let tz = self.sample(q.x, q.y);
        // Cada lectura se pasa a objeto con su base: T * x + B * y (+ N * z, que ya está en n).
        let offset = Vector3::new(0.0, 0.0, 1.0) * tx.x + Vector3::new(0.0, 1.0, 0.0) * tx.y;
        let offset = offset * w.x
            + (Vector3::new(1.0, 0.0, 0.0) * ty.x + Vector3::new(0.0, 0.0, 1.0) * ty.y) * w.y
            + (Vector3::new(1.0, 0.0, 0.0) * tz.x + Vector3::new(0.0, 1.0, 0.0) * tz.y) * w.z;
        (n + offset * strength).normalized()
    }
}

/// Ruido 4D sobre un toro: (u, v) -> (cos u, sin u, cos v, sin v). Así se repite sin costuras.
fn torus_fbm(noise: &OpenSimplex, u: f32, v: f32, freq: f32, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut f, mut norm) = (0.0, 0.5, freq, 0.0);
    for _ in 0..octaves {
        let (a, b) = (u * TAU, v * TAU);
        let p = [
            (a.cos() * f) as f64,
            (a.sin() * f) as f64,
            (b.cos() * f) as f64,
            (b.sin() * f) as f64,
        ];
        sum += noise.get(p) as f32 * amp;
        norm += amp;
        amp *= 0.5;
        f *= 2.0;
    }
    sum / norm
}

/// Roca con piedritas y golpes: fBm con un poco de "ridged" encima.
pub fn rock_map(noise: &OpenSimplex) -> NormalMap {
    NormalMap::from_height(256, 9.0, |u, v| {
        let base = torus_fbm(noise, u, v, 1.6, 5);
        let ridge = 1.0 - torus_fbm(noise, u + 0.37, v + 0.11, 3.0, 3).abs();
        base * 0.7 + ridge * ridge * 0.3
    })
}

/// Hielo agrietado: Voronoi en una rejilla que da la vuelta (grietas = fronteras de celdas).
pub fn crack_map(noise: &OpenSimplex) -> NormalMap {
    const CELLS: i32 = 9;
    NormalMap::from_height(256, 6.0, |u, v| {
        let (px, py) = (u * CELLS as f32, v * CELLS as f32);
        let (cx, cy) = (px.floor() as i32, py.floor() as i32);
        let (mut f1, mut f2) = (f32::MAX, f32::MAX);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (x, y) = (cx + dx, cy + dy);
                let (wx, wy) = (x.rem_euclid(CELLS), y.rem_euclid(CELLS));
                let fx = x as f32 + hash3(wx, wy, 3);
                let fy = y as f32 + hash3(wx, wy, 9);
                let d = ((fx - px).powi(2) + (fy - py).powi(2)).sqrt();
                if d < f1 {
                    f2 = f1;
                    f1 = d;
                } else if d < f2 {
                    f2 = d;
                }
            }
        }
        let crack = smoothstep(0.0, 0.12, f2 - f1);
        crack * 0.8 + torus_fbm(noise, u, v, 4.0, 3) * 0.2
    })
}

pub struct NormalMaps {
    pub rock: NormalMap,
    pub cracks: NormalMap,
}

impl NormalMaps {
    pub fn generate(noise: &OpenSimplex) -> Self {
        Self { rock: rock_map(noise), cracks: crack_map(noise) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_normales_son_unitarias_y_miran_hacia_afuera() {
        let map = NormalMap::from_height(32, 5.0, |u, v| (u * TAU).sin() * (v * TAU).cos());
        for &n in &map.data {
            assert!((n.length() - 1.0).abs() < 1e-4);
            assert!(n.z > 0.0);
        }
    }

    #[test]
    fn la_textura_se_repite_sin_costura() {
        let map = NormalMap::from_height(32, 5.0, |u, v| (u * TAU).sin() + (v * TAU * 2.0).cos());
        assert!((map.sample(0.0, 0.3) - map.sample(1.0, 0.3)).length() < 1e-4);
        assert!((map.sample(0.25, 0.999) - map.sample(0.25, -0.001)).length() < 1e-3);
    }
}
