//! Cargador de .obj con materiales (.mtl). Solo lee lo que usamos: v, vn, f, usemtl y Kd.
//! Los modelos son low poly, así que cada cara lleva su propia normal plana.
use crate::mesh::{Mesh, Vertex};
use raylib::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

struct Material {
    color: Vector3,
    emit: f32,
}

fn parse3(it: &mut std::str::SplitWhitespace) -> Vector3 {
    let mut n = [0.0f32; 3];
    for v in n.iter_mut() {
        *v = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    }
    Vector3::new(n[0], n[1], n[2])
}

/// Lee el .mtl. Los materiales cuyo nombre empieza con "Motor" o "Brillo" se marcan
/// como emisivos (el shader los dibuja aunque no les dé la luz).
fn load_mtl(path: &Path) -> HashMap<String, Material> {
    let mut out = HashMap::new();
    let Ok(text) = fs::read_to_string(path) else { return out };
    let mut current = String::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("newmtl") => {
                current = it.next().unwrap_or("").to_string();
                let emit = if current.starts_with("Motor") || current.starts_with("Brillo") { 1.0 } else { 0.0 };
                out.insert(current.clone(), Material { color: Vector3::one(), emit });
            }
            Some("Kd") => {
                if let Some(m) = out.get_mut(&current) {
                    m.color = parse3(&mut it);
                }
            }
            _ => {}
        }
    }
    out
}

pub fn load(path: &str) -> Result<Mesh, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("No se pudo leer {path}: {e}"))?;
    let dir = Path::new(path).parent().unwrap_or(Path::new("."));
    let mut materials = HashMap::new();
    let mut positions = Vec::new();
    let mut mesh = Mesh::default();
    let (mut color, mut emit) = (Vector3::one(), 0.0);

    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("mtllib") => {
                if let Some(name) = it.next() {
                    materials = load_mtl(&dir.join(name));
                }
            }
            Some("usemtl") => {
                if let Some(m) = it.next().and_then(|n| materials.get(n)) {
                    color = m.color;
                    emit = m.emit;
                }
            }
            Some("v") => positions.push(parse3(&mut it)),
            Some("f") => {
                let corners: Vec<Vector3> = it
                    .filter_map(|tok| {
                        let i: i64 = tok.split('/').next()?.parse().ok()?;
                        let i = if i < 0 { positions.len() as i64 + i } else { i - 1 };
                        positions.get(i as usize).copied()
                    })
                    .collect();
                // Abanico: una cara de n lados = n - 2 triángulos.
                for i in 1..corners.len().saturating_sub(1) {
                    let (a, b, c) = (corners[0], corners[i], corners[i + 1]);
                    let normal = (b - a).cross(c - a).normalized();
                    let base = mesh.vertices.len() as u32;
                    for p in [a, b, c] {
                        mesh.vertices.push(Vertex { position: p, normal, color, emit });
                    }
                    mesh.indices.push([base, base + 1, base + 2]);
                }
            }
            _ => {}
        }
    }
    if mesh.indices.is_empty() {
        return Err(format!("{path} no tiene caras"));
    }
    mesh.update_radius();
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carga_la_nave() {
        let m = load("assets/nave.obj").unwrap();
        assert_eq!(m.indices.len(), 224);
        assert!(m.radius > 4.0);
    }

    #[test]
    fn motores_marcados_como_emisivos() {
        let m = load("assets/nave.obj").unwrap();
        let motores = m.vertices.iter().filter(|v| v.emit > 0.5).count();
        assert!(motores > 0 && motores < m.vertices.len() / 2);
    }

    #[test]
    fn todos_los_modelos_de_la_superficie_cargan() {
        for f in ["arbol", "roca", "casa", "cristal", "obelisco", "hongo", "faro"] {
            let m = load(&format!("assets/{f}.obj")).unwrap_or_else(|e| panic!("{e}"));
            assert!(m.indices.len() >= 20, "{f}");
            // Todos se paran sobre y = 0: nada queda muy por debajo del suelo.
            let min_y = m.vertices.iter().map(|v| v.position.y).fold(f32::MAX, f32::min);
            assert!(min_y > -0.05, "{f}");
        }
    }
}
