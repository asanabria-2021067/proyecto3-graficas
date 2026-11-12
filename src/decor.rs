//! Objetos sobre la superficie de los planetas: árboles, rocas, casas, cristales y obeliscos.
//!
//! Distribución procedural: con una semilla se generan puntos al azar sobre la esfera
//! (siempre los mismos para la misma semilla) y cada punto se acepta o no según el mismo
//! ruido que usa el shader del planeta. Así los árboles caen en tierra húmeda, las casas
//! donde de noche hay luces de ciudad, y nada queda flotando sobre el mar o la lava.
use crate::math::{self, Mat4};
use crate::mesh::Mesh;
use crate::obj;
use crate::rng::Rng;
use crate::shaders::noise_helpers::{fbm, ridged};
use crate::shaders::{rocky, toon, Material, ShadeObject};
use crate::system::Body;
use noise::OpenSimplex;
use raylib::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Tree,
    Rock,
    House,
    Crystal,
    Obelisk,
    Mushroom,
    Lighthouse,
}

pub const KINDS: [Kind; 7] = [Kind::Tree, Kind::Rock, Kind::House, Kind::Crystal, Kind::Obelisk, Kind::Mushroom, Kind::Lighthouse];

impl Kind {
    fn file(self) -> &'static str {
        match self {
            Kind::Tree => "assets/arbol.obj",
            Kind::Rock => "assets/roca.obj",
            Kind::House => "assets/casa.obj",
            Kind::Crystal => "assets/cristal.obj",
            Kind::Obelisk => "assets/obelisco.obj",
            Kind::Mushroom => "assets/hongo.obj",
            Kind::Lighthouse => "assets/faro.obj",
        }
    }
}

pub struct Models {
    pub meshes: Vec<Mesh>, // mismo orden que KINDS
}

impl Models {
    pub fn load() -> Result<Self, String> {
        let meshes = KINDS.iter().map(|k| obj::load(k.file())).collect::<Result<Vec<_>, _>>()?;
        Ok(Self { meshes })
    }

    pub fn mesh(&self, k: Kind) -> &Mesh {
        &self.meshes[k as usize]
    }
}

/// Un objeto puesto sobre un planeta. Todo está en el espacio del planeta: así gira con él.
#[derive(Clone, Copy, Debug)]
pub struct Instance {
    pub kind: Kind,
    pub dir: Vector3, // dirección desde el centro (unitaria)
    pub yaw: f32,     // giro alrededor de la normal
    pub scale: f32,   // en fracciones del radio del planeta
}

/// Reglas de cada planeta: cuántos puntos probar y qué poner en cada uno.
fn choose(material: Material, noise: &OpenSimplex, dir: Vector3, rng: &mut Rng) -> Option<(Kind, f32)> {
    match material {
        Material::Rocky => {
            let t = rocky::terrain(noise, dir);
            if t.land < 0.95 || t.ice > 0.05 {
                return None;
            }
            if rocky::city_density(noise, dir, &t) > 0.3 && rng.f32() < 0.8 {
                return Some((Kind::House, rng.range(0.022, 0.03)));
            }
            if t.moisture > 0.5 && t.cold < 0.6 && rng.f32() < 0.85 {
                return Some((Kind::Tree, rng.range(0.028, 0.048)));
            }
            if (t.elevation > 0.1 || t.moisture < 0.42) && rng.f32() < 0.35 {
                return Some((Kind::Rock, rng.range(0.025, 0.05)));
            }
            None
        }
        Material::Lava => {
            // Solo en las partes altas, lejos de los mares de lava.
            let low = fbm(noise, dir * 1.4 + Vector3::new(9.0, 3.0, 1.0), 3);
            (low > 0.02 && rng.f32() < 0.6).then(|| (Kind::Rock, rng.range(0.04, 0.08)))
        }
        Material::Ice => {
            let sea = fbm(noise, dir * 1.5 + Vector3::new(2.0, 5.0, 1.0), 3);
            (sea > -0.08 && rng.f32() < 0.55).then(|| (Kind::Crystal, rng.range(0.04, 0.09)))
        }
        Material::Barren => {
            // Obeliscos solo en las crestas más altas; rocas en cualquier parte.
            if ridged(noise, dir * 2.5 + Vector3::new(4.0, 0.0, 0.0), 3) > 0.66 && rng.f32() < 0.12 {
                Some((Kind::Obelisk, rng.range(0.09, 0.13)))
            } else {
                (rng.f32() < 0.3).then(|| (Kind::Rock, rng.range(0.03, 0.06)))
            }
        }
        Material::Toon => {
            // Hongos y árboles fuera del camino en espiral.
            if toon::on_path(dir) {
                return None;
            }
            if rng.f32() < 0.6 {
                Some((Kind::Mushroom, rng.range(0.11, 0.18)))
            } else {
                (rng.f32() < 0.5).then(|| (Kind::Tree, rng.range(0.12, 0.17)))
            }
        }
        _ => None,
    }
}

fn attempts(material: Material) -> usize {
    match material {
        Material::Rocky => 1400,
        Material::Lava => 220,
        Material::Ice => 260,
        Material::Barren => 300,
        Material::Toon => 70,
        _ => 0,
    }
}

pub fn scatter(body: &Body, noise: &OpenSimplex, seed: u64) -> Vec<Instance> {
    let mut rng = Rng::new(seed ^ (body.name.len() as u64 * 7919) ^ body.orbit_radius as u64);
    let mut out = Vec::new();
    if body.material == Material::Toon {
        // Un solo faro, siempre en el mismo lugar cerca del polo norte.
        out.push(Instance { kind: Kind::Lighthouse, dir: Vector3::new(0.25, 0.93, 0.27).normalized(), yaw: 0.6, scale: 0.22 });
    }
    for _ in 0..attempts(body.material) {
        let dir = rng.on_sphere();
        let yaw = rng.range(0.0, std::f32::consts::TAU);
        if let Some((kind, scale)) = choose(body.material, noise, dir, &mut rng) {
            out.push(Instance { kind, dir, yaw, scale });
        }
    }
    out
}

/// Base tangente en un punto de la esfera: `up` es la normal, `east` y `north` son tangentes.
pub fn tangent_frame(up: Vector3) -> (Vector3, Vector3) {
    let helper = if up.y.abs() < 0.95 { Vector3::new(0.0, 1.0, 0.0) } else { Vector3::new(1.0, 0.0, 0.0) };
    let east = helper.cross(up).normalized();
    let north = up.cross(east);
    (east, north)
}

/// Matriz de modelo de un objeto: se para sobre la superficie con su eje Y en la normal.
/// Como la dirección está en el espacio del planeta, se pasa al mundo con el giro de este
/// cuadro: el objeto sigue pegado al mismo punto mientras el planeta rota.
/// Devuelve también el centro y el radio de su esfera envolvente (para el descarte).
pub fn model_matrix(inst: &Instance, body: &Body, planet: &ShadeObject, noise: &OpenSimplex, mesh_radius: f32) -> (Mat4, Vector3, f32) {
    let r = body.radius();
    let s = inst.scale * r;
    let surface = r * body.height(noise, inst.dir);
    let (east, north) = tangent_frame(inst.dir);
    let fwd_obj = east * inst.yaw.cos() + north * inst.yaw.sin();
    let up = planet.to_world(inst.dir);
    let forward = planet.to_world(fwd_obj);
    let right = up.cross(forward);
    // Se hunde un poquito para que la base no quede en el aire por la curvatura.
    let pos = planet.center + up * (surface - s * 0.06);
    (math::from_basis(pos, right, up, forward, s), pos + up * (s * 0.5), mesh_radius * s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::System;

    #[test]
    fn misma_semilla_mismas_posiciones() {
        let noise = OpenSimplex::new(7);
        let s = System::new();
        let a = scatter(&s.bodies[2], &noise, 5);
        let b = scatter(&s.bodies[2], &noise, 5);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(x.kind, y.kind);
            assert!((x.dir - y.dir).length() < 1e-6);
        }
    }

    #[test]
    fn otra_semilla_cambia_las_posiciones() {
        let noise = OpenSimplex::new(7);
        let s = System::new();
        let a = scatter(&s.bodies[2], &noise, 5);
        let b = scatter(&s.bodies[2], &noise, 6);
        assert!((a[0].dir - b[0].dir).length() > 1e-3);
    }
}
