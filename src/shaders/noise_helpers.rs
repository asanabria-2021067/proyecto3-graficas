//! Funciones de ruido sobre OpenSimplex.
use noise::{NoiseFn, OpenSimplex};
use raylib::prelude::*;

/// Ruido 3D, aprox. en [-1, 1].
pub fn sample(noise: &OpenSimplex, p: Vector3) -> f32 {
    noise.get([p.x as f64, p.y as f64, p.z as f64]) as f32
}

/// Ruido 4D: la cuarta coordenada es el tiempo, así el patrón "hierve" en su lugar
/// en vez de solo deslizarse.
pub fn sample4(noise: &OpenSimplex, p: Vector3, w: f32) -> f32 {
    noise.get([p.x as f64, p.y as f64, p.z as f64, w as f64]) as f32
}

/// fBm: suma de octavas, cada una con el doble de frecuencia y la mitad de amplitud.
pub fn fbm(noise: &OpenSimplex, p: Vector3, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 0.5, 1.0, 0.0);
    for _ in 0..octaves {
        sum += sample(noise, p * freq) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.03;
    }
    sum / norm
}

pub fn fbm4(noise: &OpenSimplex, p: Vector3, w: f32, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 0.5, 1.0, 0.0);
    for _ in 0..octaves {
        sum += sample4(noise, p * freq, w * freq.sqrt()) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.03;
    }
    sum / norm
}

/// Ruido "ridged": 1 - |ruido|, elevado al cuadrado. Da crestas finas, como cordilleras.
pub fn ridged(noise: &OpenSimplex, p: Vector3, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 0.5, 1.0, 0.0);
    for _ in 0..octaves {
        let r = 1.0 - sample(noise, p * freq).abs();
        sum += r * r * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.1;
    }
    sum / norm
}

/// Hash de 3 enteros a un número en [0, 1). Sirve para poner un punto en cada celda.
#[inline]
pub fn hash3(x: i32, y: i32, z: i32) -> f32 {
    let mut h =
        (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ (z as u32).wrapping_mul(0xcb1a_b31f);
    h = (h ^ (h >> 13)).wrapping_mul(0x5bd1_e995);
    ((h ^ (h >> 15)) & 0x00ff_ffff) as f32 / 16_777_216.0
}

/// Ruido celular (Worley / Voronoi) en 3D. Cada celda del espacio tiene un punto al azar;
/// se devuelve la distancia al punto más cercano (F1) y al segundo más cercano (F2).
/// F2 - F1 es casi 0 en la frontera entre dos celdas: así salen las grietas.
pub fn cellular(p: Vector3) -> (f32, f32) {
    let (cx, cy, cz) = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
    let (mut f1, mut f2) = (f32::MAX, f32::MAX);
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (x, y, z) = (cx + dx, cy + dy, cz + dz);
                let feature = Vector3::new(
                    x as f32 + hash3(x, y, z),
                    y as f32 + hash3(x + 17, y - 31, z + 7),
                    z as f32 + hash3(x - 5, y + 11, z - 23),
                );
                let q = feature - p;
                let d = q.dot(q);
                if d < f1 {
                    f2 = f1;
                    f1 = d;
                } else if d < f2 {
                    f2 = d;
                }
            }
        }
    }
    (f1.sqrt(), f2.sqrt())
}
