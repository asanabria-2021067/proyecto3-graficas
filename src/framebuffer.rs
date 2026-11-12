//! Framebuffer con G-buffer.
//!
//! El render va en dos pasos: primero se rasteriza todo y el depth test se queda con el
//! fragmento más cercano de cada píxel (se guarda en `gbuf`), y después el fragment shader
//! corre una sola vez por píxel visible. Así los shaders pesados no se calculan para
//! fragmentos que luego quedan tapados.
use raylib::prelude::*;

/// Lo que se guarda de cada píxel después del depth test (lo que recibe el fragment shader).
#[derive(Clone, Copy, Debug)]
pub struct GSample {
    pub world: Vector3,  // posición en el mundo
    pub normal: Vector3, // normal interpolada en el mundo
    pub obj: Vector3,    // posición en el espacio del objeto (para los patrones de ruido)
    pub color: Vector3,  // color del vértice (material del .obj)
    pub emit: f32,
    pub id: u16, // qué objeto quedó en este píxel (0 = fondo)
}

impl GSample {
    pub const EMPTY: GSample = GSample {
        world: Vector3 { x: 0.0, y: 0.0, z: 0.0 },
        normal: Vector3 { x: 0.0, y: 0.0, z: 0.0 },
        obj: Vector3 { x: 0.0, y: 0.0, z: 0.0 },
        color: Vector3 { x: 0.0, y: 0.0, z: 0.0 },
        emit: 0.0,
        id: 0,
    };
}

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub color: Vec<Vector3>,
    /// Profundidad guardada como 1/w (w = distancia en la vista). Más grande = más cerca.
    /// 0 significa que no hay nada (infinito). Con 1/w la precisión es buena tanto cerca
    /// de la nave como lejos, en los planetas del fondo.
    pub depth: Vec<f32>,
    pub gbuf: Vec<GSample>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let n = width * height;
        Self { width, height, color: vec![Vector3::zero(); n], depth: vec![0.0; n], gbuf: vec![GSample::EMPTY; n] }
    }

    pub fn clear(&mut self) {
        self.depth.fill(0.0);
        self.gbuf.fill(GSample::EMPTY);
    }

    /// Mezcla un color encima del píxel (para líneas, HUD y brillos).
    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, c: Vector3, alpha: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let p = &mut self.color[y as usize * self.width + x as usize];
        *p = *p + (c - *p) * alpha;
    }

    pub fn to_rgba(&self, out: &mut Vec<u8>) {
        out.clear();
        out.reserve(self.width * self.height * 4);
        for c in &self.color {
            for v in [c.x, c.y, c.z] {
                out.push((v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
            }
            out.push(255);
        }
    }

    pub fn save_png(&self, path: &str) {
        let mut rgba = Vec::new();
        self.to_rgba(&mut rgba);
        let img = image::RgbaImage::from_raw(self.width as u32, self.height as u32, rgba).expect("tamaño de imagen");
        if let Some(dir) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        img.save(path).expect("no se pudo guardar el PNG");
    }
}
