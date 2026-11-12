//! Descarte de objetos fuera de la vista (frustum culling) antes de mandarlos al pipeline.
//!
//! Los 6 planos de la pirámide de visión salen directo de la matriz vista-proyección
//! (método de Gribb y Hartmann): cada plano es la suma o resta de la fila 4 con otra fila.
//! Cada objeto tiene una esfera envolvente; si la esfera queda toda detrás de algún plano,
//! el objeto no se dibuja y ni siquiera se corre su vertex shader.
use crate::math::Mat4;
use raylib::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub n: Vector3,
    pub d: f32,
}

impl Plane {
    fn from_row(r: [f32; 4]) -> Plane {
        let n = Vector3::new(r[0], r[1], r[2]);
        let len = n.length().max(1e-12);
        Plane { n: n * (1.0 / len), d: r[3] / len }
    }

    /// Distancia con signo: positiva del lado de adentro.
    pub fn distance(&self, p: Vector3) -> f32 {
        self.n.dot(p) + self.d
    }
}

pub struct Frustum {
    pub planes: [Plane; 6],
}

impl Frustum {
    pub fn from_matrix(m: &Mat4) -> Frustum {
        let row = |i: usize| m[i];
        let add = |a: [f32; 4], b: [f32; 4], s: f32| [a[0] + s * b[0], a[1] + s * b[1], a[2] + s * b[2], a[3] + s * b[3]];
        let w = row(3);
        Frustum {
            planes: [
                Plane::from_row(add(w, row(0), 1.0)),  // izquierda
                Plane::from_row(add(w, row(0), -1.0)), // derecha
                Plane::from_row(add(w, row(1), 1.0)),  // abajo
                Plane::from_row(add(w, row(1), -1.0)), // arriba
                Plane::from_row(add(w, row(2), 1.0)),  // cerca
                Plane::from_row(add(w, row(2), -1.0)), // lejos
            ],
        }
    }

    pub fn sphere_visible(&self, center: Vector3, radius: f32) -> bool {
        self.planes.iter().all(|p| p.distance(center) > -radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math;

    fn frustum() -> Frustum {
        let view = math::look_at(Vector3::zero(), Vector3::new(0.0, 0.0, -1.0), Vector3::new(0.0, 1.0, 0.0));
        let proj = math::perspective(1.0, 1.0, 0.1, 100.0);
        Frustum::from_matrix(&math::mul(&proj, &view))
    }

    #[test]
    fn esfera_enfrente_se_ve() {
        assert!(frustum().sphere_visible(Vector3::new(0.0, 0.0, -10.0), 1.0));
    }
}
