//! Malla indexada: los vértices se transforman una sola vez y los triángulos los reutilizan.
use raylib::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: Vector3,
    pub normal: Vector3,
    pub color: Vector3, // color del material (Kd del .mtl)
    pub emit: f32,      // 0 = normal, 1 = brilla solo (motores, cristales)
}

impl Vertex {
    pub fn new(position: Vector3, normal: Vector3) -> Self {
        Self { position, normal, color: Vector3::one(), emit: 0.0 }
    }
}

#[derive(Clone, Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<[u32; 3]>,
    pub radius: f32, // distancia máxima al origen (para la esfera envolvente)
}

impl Mesh {
    pub fn update_radius(&mut self) {
        self.radius = self.vertices.iter().map(|v| v.position.length()).fold(0.0, f32::max);
    }

    /// Normales suaves: promedio de las normales de las caras que tocan cada vértice.
    pub fn compute_smooth_normals(&mut self) {
        let mut acc = vec![Vector3::zero(); self.vertices.len()];
        for t in &self.indices {
            let [a, b, c] = t.map(|i| self.vertices[i as usize].position);
            let n = (b - a).cross(c - a); // sin normalizar: pesa por área
            for i in t {
                acc[*i as usize] += n;
            }
        }
        for (v, n) in self.vertices.iter_mut().zip(acc) {
            v.normal = if n.length() > 1e-12 { n.normalized() } else { v.position.normalized() };
        }
    }
}
