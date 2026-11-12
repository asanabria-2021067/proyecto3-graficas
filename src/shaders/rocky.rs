//! Tessara: planeta rocoso (viene del lab 6). Océanos, biomas, casquetes de hielo,
//! nubes que se mueven, sombras de nubes, reflejo del sol en el agua y ciudades de noche.
//! El relieve ahora sale del mapa de normales en vez de derivar el ruido en cada píxel.
use super::noise_helpers::{fbm, ridged, sample};
use super::{mix, rgb, smoothstep, surface, Globals, ShadeObject};
use crate::framebuffer::GSample;
use crate::math::rotate_y;
use noise::OpenSimplex;
use raylib::prelude::*;

pub const SEA_LEVEL: f32 = 0.04;
const CLOUD_HEIGHT: f32 = 0.06;

/// Forma de los continentes: fBm deformado por otro fBm (domain warping) para que
/// las costas se vean orgánicas y no como manchas redondas.
pub fn continents(nz: &OpenSimplex, p: Vector3) -> f32 {
    let q = p * 1.3;
    let warp = Vector3::new(
        fbm(nz, q + Vector3::new(3.1, 0.0, 0.0), 2),
        fbm(nz, q + Vector3::new(0.0, 7.4, 0.0), 2),
        fbm(nz, q + Vector3::new(0.0, 0.0, 1.9), 2),
    );
    fbm(nz, q + warp * 0.9, 5)
}

/// Lo que se sabe de un punto del terreno. Lo usan el shader y también la distribución
/// de árboles y casas, así los modelos caen donde el shader pinta tierra.
pub struct Terrain {
    pub height: f32,    // valor del ruido de continentes
    pub land: f32,      // 0 = océano, 1 = tierra
    pub elevation: f32, // altura sobre el nivel del mar
    pub moisture: f32,  // 0 = seco, 1 = húmedo
    pub cold: f32,      // latitud + altura
    pub ice: f32,       // casquete polar
}

pub fn terrain(nz: &OpenSimplex, p: Vector3) -> Terrain {
    let h = continents(nz, p);
    let land = smoothstep(SEA_LEVEL - 0.008, SEA_LEVEL + 0.008, h);
    let elevation = (h - SEA_LEVEL).max(0.0);
    let moisture = fbm(nz, p * 2.4 + Vector3::new(8.0, 2.0, 5.0), 3) * 0.5 + 0.5;
    let cold = p.y.abs() + elevation * 1.4 + sample(nz, p * 4.0) * 0.06;
    let ice = smoothstep(0.80, 0.86, p.y.abs() + fbm(nz, p * 3.5, 2) * 0.12);
    Terrain { height: h, land, elevation, moisture, cold, ice }
}

/// Zonas con ciudades: cerca de la costa, sin hielo y no muy frías.
pub fn city_density(nz: &OpenSimplex, p: Vector3, t: &Terrain) -> f32 {
    let coast = 1.0 - smoothstep(0.06, 0.22, t.elevation);
    let habitable = t.land * coast * (1.0 - t.ice) * (1.0 - smoothstep(0.6, 0.75, t.cold));
    habitable * smoothstep(-0.05, 0.35, fbm(nz, p * 7.0 + Vector3::new(4.0, 4.0, 4.0), 2))
}

/// Máscara de nubes. Giran sobre Y más rápido que el terreno y se deforman con el tiempo.
pub fn cloud_mask(nz: &OpenSimplex, p: Vector3, time: f32, coverage: f32) -> f32 {
    let pc = rotate_y(p, time * 0.035);
    let swirl = fbm(nz, pc * 1.8 + Vector3::new(0.0, time * 0.015, 0.0), 2);
    let q = pc * 3.0 + Vector3::new(swirl, swirl * 0.4, -swirl) * 1.2;
    let c = fbm(nz, Vector3::new(q.x, q.y * 1.8, q.z), 4) * 0.5 + 0.5;
    let start = 1.0 - coverage;
    smoothstep(start - 0.04, start + 0.09, c)
}

pub fn rocky(s: &GSample, o: &ShadeObject, g: &Globals) -> Vector3 {
    let sf = surface(s, o, g);
    let (p, n, l, v) = (sf.p, sf.n, sf.l, sf.v);
    let nz = g.noise;
    let coverage = o.intensity;

    // ---------- 1. Regiones de la superficie ----------
    let tr = terrain(nz, p);
    let mountains = smoothstep(0.04, 0.22, tr.elevation);
    let ridge = ridged(nz, p * 5.0, 3) * mountains;

    let desert = rgb(0.80, 0.62, 0.38);
    let grass = rgb(0.36, 0.52, 0.22);
    let forest = rgb(0.10, 0.30, 0.16);
    let tundra = rgb(0.52, 0.50, 0.42);
    let rock = rgb(0.40, 0.33, 0.28);
    let snow = rgb(0.94, 0.96, 1.0);
    let mut land_col = mix(desert, grass, smoothstep(0.38, 0.55, tr.moisture));
    land_col = mix(land_col, forest, smoothstep(0.58, 0.75, tr.moisture));
    land_col = mix(land_col, tundra, smoothstep(0.55, 0.72, tr.cold));
    land_col = mix(land_col, rock, smoothstep(0.35, 0.75, ridge));
    land_col = mix(land_col, snow, smoothstep(0.62, 0.8, ridge) * smoothstep(0.35, 0.55, tr.cold));

    let depth = (SEA_LEVEL - tr.height).max(0.0);
    let ocean_col = mix(rgb(0.08, 0.50, 0.58), rgb(0.02, 0.09, 0.26), smoothstep(0.0, 0.18, depth));

    let mut albedo = mix(ocean_col, land_col, tr.land);
    albedo = mix(albedo, rgb(0.90, 0.95, 1.0), tr.ice);

    // ---------- 2. Relieve con el mapa de normales (solo en tierra) ----------
    let ndl_geo = n.dot(l);
    let shading_n = if g.normal_mapping && tr.land > 0.0 {
        let strength = (0.35 + mountains * 0.9) * tr.land * (1.0 - tr.ice);
        g.maps.rock.triplanar(p, n, 3.0, strength)
    } else {
        n
    };

    // ---------- 3. Nubes y sus sombras ----------
    let clouds = cloud_mask(nz, p, g.time, coverage);
    let shadow = if ndl_geo > -0.1 {
        // Rayo del suelo hacia el sol: ¿dónde cruza la capa de nubes (radio 1 + altura)?
        // |p + t·l|² = r²  ->  t = -(p·l) + sqrt((p·l)² + r² - 1)
        let r = 1.0 + CLOUD_HEIGHT;
        let pl = p.dot(l);
        let t = -pl + (pl * pl + r * r - 1.0).sqrt();
        1.0 - cloud_mask(nz, (p + l * t).normalized(), g.time, coverage) * 0.8
    } else {
        1.0
    };

    // ---------- 4. Luz ----------
    let diffuse = shading_n.dot(l).max(0.0) * smoothstep(-0.05, 0.1, ndl_geo);
    let mut color = albedo * (0.025 + diffuse * shadow * 1.05);

    // Blinn-Phong: el océano brilla concentrado, el hielo un poco, la tierra nada.
    let half = (l + v).normalized();
    let ndh = n.dot(half).max(0.0);
    let ocean = (1.0 - tr.land) * (1.0 - tr.ice);
    let spec = ndh.powf(90.0) * 1.1 * ocean + ndh.powf(14.0) * 0.18 * tr.ice;
    color = color + rgb(1.0, 0.95, 0.85) * (spec * smoothstep(0.0, 0.08, ndl_geo) * shadow);

    // ---------- 5. Lado de noche: luces de ciudades ----------
    let day = smoothstep(-0.12, 0.18, ndl_geo);
    if day < 1.0 {
        let dots = smoothstep(0.2, 0.6, sample(nz, p * 70.0));
        let city = city_density(nz, p, &tr) * (0.35 + dots * 0.9);
        color = color + rgb(1.0, 0.72, 0.32) * (city * (1.0 - day) * (1.0 - clouds * 0.85) * 1.6);
    }

    // Nubes encima de todo.
    let cloud_light = 0.03 + ndl_geo.max(0.0) * 1.05;
    color = mix(color, rgb(0.97, 0.98, 1.0) * cloud_light, clouds * 0.92);

    // Atmósfera: halo azul en el borde del lado iluminado.
    let rim = (1.0 - n.dot(v).max(0.0)).powf(3.0);
    color + rgb(0.30, 0.55, 1.0) * (rim * smoothstep(-0.25, 0.5, ndl_geo) * 0.7)
}
