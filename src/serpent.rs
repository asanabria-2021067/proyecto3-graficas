//! La serpiente de roca que sale de la superficie de Tessara.
//!
//! El cuerpo se arma cada cuadro: es una cadena de segmentos (rocas deformadas con ruido)
//! puestos sobre una curva en espiral que sale del suelo. La cabeza avanza por la curva,
//! así la serpiente va "emergiendo" con el tiempo, se queda ondulando y vuelve a hundirse.
//! Todo se calcula en el espacio del planeta, así gira con él.
use crate::decor::tangent_frame;
use crate::geometry;
use crate::math::{self, Mat4};
use crate::mesh::Mesh;
use crate::shaders::rocky;
use crate::shaders::ShadeObject;
use crate::system::Body;
use noise::OpenSimplex;
use raylib::prelude::*;

pub const SEGMENTS: usize = 40;
const SPACING: f32 = 0.034; // separación entre segmentos sobre la curva (en radios del planeta)
const CYCLE: f32 = 48.0;    // segundos: sale, se queda y se hunde

pub struct Serpent {
    pub body: usize,
    pub dir: Vector3, // dónde sale, en el espacio del planeta
    pub segment: Mesh,
    pub head: Mesh,
    pub eye: Mesh,
}

/// Una parte del cuerpo lista para dibujar.
pub struct Part<'a> {
    pub mesh: &'a Mesh,
    pub model: Mat4,
    pub center: Vector3,
    pub radius: f32,
    pub param: f32, // 0 = cabeza ... 1 = cola, -1 = ojo
}

impl Serpent {
    pub fn new(body: usize, noise: &OpenSimplex) -> Self {
        // Busca un punto de tierra cerca del ecuador (el mismo siempre: puntos de Fibonacci).
        let golden = std::f32::consts::PI * (3.0 - 5f32.sqrt());
        let dir = (0..400)
            .map(|i| {
                let y = 1.0 - (i as f32 + 0.5) / 400.0 * 2.0;
                let r = (1.0 - y * y).sqrt();
                Vector3::new(r * (golden * i as f32).cos(), y, r * (golden * i as f32).sin())
            })
            .find(|d| {
                let t = rocky::terrain(noise, *d);
                d.y.abs() < 0.35 && t.land > 0.99 && t.elevation > 0.03 && t.elevation < 0.15 && t.ice < 0.01
            })
            .unwrap_or(Vector3::new(0.0, 0.0, 1.0));

        // Segmento: icosfera chiquita deformada con ruido para que parezca roca.
        let rock = |seed: f32, stretch: Vector3| {
            let base = geometry::icosphere(2);
            let mut m = geometry::deform(&base, |d| 1.0 + crate::shaders::noise_helpers::fbm(noise, d * 2.2 + Vector3::new(seed, 0.0, 0.0), 3) * 0.35);
            for v in m.vertices.iter_mut() {
                v.position = Vector3::new(v.position.x * stretch.x, v.position.y * stretch.y, v.position.z * stretch.z);
            }
            m.compute_smooth_normals();
            m.update_radius();
            m
        };
        Self {
            body,
            dir,
            segment: rock(3.0, Vector3::new(1.0, 0.85, 1.25)),
            head: rock(9.0, Vector3::new(0.9, 0.75, 1.7)),
            eye: geometry::icosphere(1),
        }
    }

    /// Cuánto de la serpiente ya salió (largo de la curva hasta la cabeza).
    fn head_length(time: f32) -> f32 {
        let c = time.rem_euclid(CYCLE) / CYCLE;
        let full = SEGMENTS as f32 * SPACING + 0.1;
        let rise = (c / 0.3).min(1.0);
        let sink = ((c - 0.82) / 0.18).clamp(0.0, 1.0);
        let ease = |x: f32| x * x * (3.0 - 2.0 * x);
        full * (ease(rise) - ease(sink))
    }

    /// Curva en el espacio tangente del punto de salida: sube dando vueltas.
    /// `u` = largo recorrido desde el suelo. Devuelve (x, altura, z) en radios del planeta.
    fn curve(u: f32, time: f32) -> Vector3 {
        // La ondulación crece con la altura: la base se queda fija en el suelo.
        let sway = (time * 0.7 + u * 3.0).sin() * 0.06 * (u * 2.0).min(1.0);
        let coil = 0.16 + u * 0.1;
        let a = u * 4.2 + sway;
        Vector3::new(a.cos() * coil - coil, u * 0.42, a.sin() * coil)
    }

    pub fn parts<'a>(&'a self, body: &Body, planet: &ShadeObject, time: f32) -> Vec<Part<'a>> {
        let r = body.radius();
        let (east, north) = tangent_frame(self.dir);
        let up_w = planet.to_world(self.dir);
        let east_w = planet.to_world(east);
        let north_w = planet.to_world(north);
        let base = planet.center + up_w * (r * 0.98);
        let to_world = |c: Vector3| base + (east_w * c.x + up_w * c.y + north_w * c.z) * r;

        let head_u = Self::head_length(time);
        let mut out = Vec::with_capacity(SEGMENTS + 2);
        for k in 0..SEGMENTS {
            let u = head_u - k as f32 * SPACING;
            if u < -0.05 {
                break; // todavía bajo tierra
            }
            let p = to_world(Self::curve(u, time));
            let ahead = to_world(Self::curve(u + 0.01, time));
            let forward = (ahead - p).normalized();
            let side = math::reject(up_w, forward);
            let up = if side.length() > 1e-3 { side.normalized() } else { east_w };
            let right = up.cross(forward);
            let param = k as f32 / (SEGMENTS - 1) as f32;
            let size = r * 0.068 * (1.0 - param * 0.5);
            let is_head = k == 0;
            let mesh = if is_head { &self.head } else { &self.segment };
            let size = if is_head { size * 1.25 } else { size };
            out.push(Part { mesh, model: math::from_basis(p, right, up, forward, size), center: p, radius: mesh.radius * size, param });
            if is_head {
                // Dos ojos a los lados de la cabeza.
                for s in [-1.0, 1.0] {
                    let e = p + right * (size * 0.55 * s) + up * (size * 0.35) + forward * (size * 0.9);
                    let es = size * 0.16;
                    out.push(Part { mesh: &self.eye, model: math::from_basis(e, right, up, forward, es), center: e, radius: es, param: -1.0 });
                }
            }
        }
        out
    }

    /// Esfera que envuelve a toda la serpiente (para descartarla de una vez).
    pub fn bounds(&self, body: &Body, planet: &ShadeObject) -> (Vector3, f32) {
        let r = body.radius();
        (planet.center + planet.to_world(self.dir) * (r * 1.3), r * 0.9)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sale_se_queda_y_se_hunde() {
        let full = SEGMENTS as f32 * SPACING + 0.1;
        assert!(Serpent::head_length(0.0).abs() < 1e-4);
        assert!((Serpent::head_length(CYCLE * 0.5) - full).abs() < 1e-4);
        assert!(Serpent::head_length(CYCLE * 0.999) < 0.01);
    }

    #[test]
    fn la_curva_sale_del_suelo_y_sube() {
        assert!(Serpent::curve(0.0, 3.0).length() < 1e-4);
        let mut prev = 0.0;
        for k in 1..20 {
            let h = Serpent::curve(k as f32 * 0.07, 3.0).y;
            assert!(h > prev);
            prev = h;
        }
    }

    #[test]
    fn sale_en_tierra_de_tessara() {
        let noise = OpenSimplex::new(7);
        let s = Serpent::new(2, &noise);
        assert!(rocky::terrain(&noise, s.dir).land > 0.99);
        let sys = crate::system::System::new();
        let b = &sys.bodies[2];
        let parts = s.parts(b, &b.shade_object(0.0), CYCLE * 0.5);
        // Todos los segmentos + 2 ojos cuando ya salió completa.
        assert_eq!(parts.len(), SEGMENTS + 2);
    }
}
