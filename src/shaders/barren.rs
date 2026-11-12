//! Kharon: planeta pequeño y sin atmósfera. Es el único con la geometría deformada:
//! los vértices de la icosfera se mueven con ruido (montañas) y con cráteres.
//! El shader colorea según la altura real de cada punto y la pendiente de la malla.
//! `intensity` = fuerza del mapa de normales.
use super::noise_helpers::{fbm, hash3, ridged};
use super::{mix, ramp, rgb, smoothstep, surface, Globals, ShadeObject};
use crate::framebuffer::GSample;
use noise::OpenSimplex;
use raylib::prelude::*;

const CRATERS: i32 = 16;

/// Cráter: hoyo en forma de tazón con un borde levantado alrededor.
/// `d` = distancia angular al centro, `r` = radio del cráter.
fn crater_profile(d: f32, r: f32, depth: f32) -> f32 {
    let x = d / r;
    let bowl = if x < 1.0 { (x * x - 1.0) * depth } else { 0.0 };
    let rim = depth * 0.5 * (-((x - 1.0) / 0.22).powi(2)).exp();
    bowl + rim
}

/// Radio del planeta en la dirección `dir` (1.0 = esfera original). Lo usan la deformación
/// de la malla, las colisiones de la nave y la colocación de objetos sobre la superficie.
pub fn height(nz: &OpenSimplex, dir: Vector3) -> f32 {
    let mut h = 1.0 + fbm(nz, dir * 1.6, 4) * 0.07 + ridged(nz, dir * 2.5 + Vector3::new(4.0, 0.0, 0.0), 3) * 0.06;
    for i in 0..CRATERS {
        // Centro y tamaño de cada cráter salen de un hash: siempre los mismos.
        let c = Vector3::new(hash3(i, 1, 7) * 2.0 - 1.0, hash3(i, 2, 7) * 2.0 - 1.0, hash3(i, 3, 7) * 2.0 - 1.0);
        if c.length() < 0.1 {
            continue;
        }
        let c = c.normalized();
        let r = 0.12 + hash3(i, 4, 7) * 0.25;
        let d = dir.dot(c).clamp(-1.0, 1.0).acos();
        if d < r * 1.8 {
            h += crater_profile(d, r, 0.035 + r * 0.08);
        }
    }
    h
}

pub fn barren(s: &GSample, o: &ShadeObject, g: &Globals) -> Vector3 {
    let sf = surface(s, o, g);
    let (p, n, l, v) = (sf.p, sf.n, sf.l, sf.v);
    let nz = g.noise;

    // La posición en el objeto conserva la deformación: su largo es la altura.
    let h = s.obj.length();
    let slope = 1.0 - n.dot(p).max(0.0); // 0 = plano, crece en laderas

    let mut albedo = ramp(
        &[
            (0.90, rgb(0.16, 0.12, 0.18)),
            (0.97, rgb(0.34, 0.28, 0.33)),
            (1.03, rgb(0.55, 0.50, 0.52)),
            (1.10, rgb(0.78, 0.74, 0.70)),
        ],
        h,
    );
    // Polvo morado acumulado en lo plano, roca más clara en las laderas.
    let dust = fbm(nz, p * 6.0, 3) * 0.5 + 0.5;
    albedo = mix(albedo, rgb(0.30, 0.22, 0.36), smoothstep(0.4, 0.8, dust) * (1.0 - slope * 4.0).max(0.0) * 0.5);
    albedo = mix(albedo, rgb(0.62, 0.58, 0.55), smoothstep(0.05, 0.25, slope) * 0.5);

    let shading_n = if g.normal_mapping { g.maps.rock.triplanar(p, n, 5.0, 0.4 + o.intensity) } else { n };

    // Sin atmósfera: la sombra es muy marcada y casi no hay luz ambiente.
    let diffuse = shading_n.dot(l).max(0.0) * smoothstep(-0.02, 0.06, n.dot(l));
    let half = (l + v).normalized();
    let spec = shading_n.dot(half).max(0.0).powf(16.0) * 0.08 * diffuse;
    albedo * (0.012 + diffuse * 1.1) + rgb(1.0, 0.95, 0.9) * spec
}
