//! Generación procedural de geometría antes de rasterizar.
//!
//! Los planetas no vienen de un .obj: se arman desde un icosaedro (20 triángulos) y se
//! subdividen. En cada nivel cada triángulo se parte en 4 usando los puntos medios de sus
//! aristas, y esos puntos se empujan a la superficie de la esfera.
//! Nivel 0 = 20 triángulos, nivel 1 = 80, nivel 2 = 320 ... nivel n = 20 * 4^n.
use crate::mesh::{Mesh, Vertex};
use raylib::prelude::*;
use std::collections::HashMap;

pub fn icosahedron() -> Mesh {
    let t = (1.0 + 5f32.sqrt()) / 2.0;
    let raw = [
        (-1.0, t, 0.0), (1.0, t, 0.0), (-1.0, -t, 0.0), (1.0, -t, 0.0),
        (0.0, -1.0, t), (0.0, 1.0, t), (0.0, -1.0, -t), (0.0, 1.0, -t),
        (t, 0.0, -1.0), (t, 0.0, 1.0), (-t, 0.0, -1.0), (-t, 0.0, 1.0),
    ];
    let vertices = raw
        .iter()
        .map(|&(x, y, z)| {
            let p = Vector3::new(x, y, z).normalized();
            Vertex::new(p, p)
        })
        .collect();
    let indices = vec![
        [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
        [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
        [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
        [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
    ];
    Mesh { vertices, indices, radius: 1.0 }
}

/// Un paso de subdivisión: cada triángulo -> 4 triángulos.
/// El HashMap guarda el punto medio de cada arista para que dos triángulos vecinos
/// compartan el mismo vértice (si no, quedarían grietas).
pub fn subdivide(mesh: &Mesh) -> Mesh {
    let mut vertices = mesh.vertices.clone();
    let mut cache: HashMap<(u32, u32), u32> = HashMap::new();
    let mut midpoint = |a: u32, b: u32, vertices: &mut Vec<Vertex>| -> u32 {
        let key = (a.min(b), a.max(b));
        if let Some(&i) = cache.get(&key) {
            return i;
        }
        let p = ((vertices[a as usize].position + vertices[b as usize].position) * 0.5).normalized();
        vertices.push(Vertex::new(p, p));
        let i = vertices.len() as u32 - 1;
        cache.insert(key, i);
        i
    };
    let mut indices = Vec::with_capacity(mesh.indices.len() * 4);
    for &[a, b, c] in &mesh.indices {
        let ab = midpoint(a, b, &mut vertices);
        let bc = midpoint(b, c, &mut vertices);
        let ca = midpoint(c, a, &mut vertices);
        indices.push([a, ab, ca]);
        indices.push([b, bc, ab]);
        indices.push([c, ca, bc]);
        indices.push([ab, bc, ca]);
    }
    Mesh { vertices, indices, radius: 1.0 }
}

/// Esfera de radio 1 con `level` subdivisiones.
pub fn icosphere(level: u32) -> Mesh {
    let mut m = icosahedron();
    for _ in 0..level {
        m = subdivide(&m);
    }
    m
}

/// Todos los niveles de detalle de una esfera, del 0 al `max_level`.
pub fn icosphere_lods(max_level: u32) -> Vec<Mesh> {
    let mut out = vec![icosahedron()];
    for _ in 0..max_level {
        let next = subdivide(out.last().unwrap());
        out.push(next);
    }
    out
}

/// Deformación: cada vértice se mueve sobre su normal según `height(dir)`
/// (1.0 = sin cambio). Después se recalculan las normales para que la luz siga la forma nueva.
pub fn deform(mesh: &Mesh, height: impl Fn(Vector3) -> f32) -> Mesh {
    let mut out = mesh.clone();
    for v in out.vertices.iter_mut() {
        let dir = v.position.normalized();
        v.position = dir * height(dir);
    }
    out.compute_smooth_normals();
    out.update_radius();
    out
}

/// Anillo plano (en el plano XZ) entre `inner` y `outer`, visto por las dos caras.
pub fn ring(inner: f32, outer: f32, segments: u32) -> Mesh {
    let mut mesh = Mesh::default();
    let up = Vector3::new(0.0, 1.0, 0.0);
    for i in 0..=segments {
        let a = i as f32 / segments as f32 * std::f32::consts::TAU;
        let d = Vector3::new(a.cos(), 0.0, a.sin());
        mesh.vertices.push(Vertex::new(d * inner, up));
        mesh.vertices.push(Vertex::new(d * outer, up));
    }
    for i in 0..segments {
        let k = i * 2;
        mesh.indices.push([k, k + 1, k + 3]);
        mesh.indices.push([k, k + 3, k + 2]);
    }
    mesh.update_radius();
    mesh
}
