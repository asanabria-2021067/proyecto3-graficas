//! Pipeline de software: vertex shader -> recorte contra el plano cercano -> rasterizado
//! con baricéntricas (perspectiva correcta) -> depth test -> G-buffer.
//! El fragment shader corre después, en `render.rs`, solo sobre los píxeles visibles.
use crate::framebuffer::{Framebuffer, GSample};
use crate::math::{self, Mat4};
use crate::mesh::{Mesh, Vertex};
use raylib::prelude::*;
use rayon::prelude::*;

/// Contadores para demostrar qué tanto trabajo se ahorra con el descarte y el LOD.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub objects: u32,
    pub objects_culled: u32,
    pub triangles_in: u32,
    pub triangles_frustum: u32, // descartados por estar fuera de la vista
    pub triangles_back: u32,    // descartados por estar de espaldas
    pub triangles_drawn: u32,
    pub vertices: u32,
}

/// Lo que se le pide al pipeline para dibujar un objeto.
pub struct DrawCall<'a> {
    pub mesh: &'a Mesh,
    pub model: Mat4, // objeto -> mundo (escala uniforme, así la normal se transforma igual)
    pub id: u16,     // con este id el fragment shader sabe qué material usar
    pub double_sided: bool,
    /// Prueba de transparencia por posición en el objeto (para los huecos del anillo).
    pub alpha: Option<fn(Vector3) -> bool>,
}

/// Vértice después del vertex shader, todavía en coordenadas de recorte (clip space).
#[derive(Clone, Copy)]
struct ClipVertex {
    clip: [f32; 4],
    world: Vector3,
    normal: Vector3,
    obj: Vector3,
    color: Vector3,
    emit: f32,
}

impl ClipVertex {
    fn lerp(&self, o: &ClipVertex, t: f32) -> ClipVertex {
        let l = |a: Vector3, b: Vector3| a + (b - a) * t;
        let mut clip = [0.0; 4];
        for (i, c) in clip.iter_mut().enumerate() {
            *c = self.clip[i] + (o.clip[i] - self.clip[i]) * t;
        }
        ClipVertex {
            clip,
            world: l(self.world, o.world),
            normal: l(self.normal, o.normal),
            obj: l(self.obj, o.obj),
            color: l(self.color, o.color),
            emit: self.emit + (o.emit - self.emit) * t,
        }
    }

    /// Bits que dicen de qué planos del volumen de vista se sale el vértice.
    fn outcode(&self) -> u8 {
        let [x, y, z, w] = self.clip;
        (x < -w) as u8
            | ((x > w) as u8) << 1
            | ((y < -w) as u8) << 2
            | ((y > w) as u8) << 3
            | ((z < -w) as u8) << 4
            | ((z > w) as u8) << 5
    }
}

/// Vertex shader: modelo -> mundo -> vista -> proyección.
fn vertex_shader(v: &Vertex, model: &Mat4, mvp: &Mat4) -> ClipVertex {
    ClipVertex {
        clip: math::transform(mvp, v.position, 1.0),
        world: math::transform_point(model, v.position),
        normal: math::transform_dir(model, v.normal).normalized(),
        obj: v.position,
        color: v.color,
        emit: v.emit,
    }
}

/// Vértice ya en pantalla. Los atributos se guardan multiplicados por 1/w para
/// interpolarlos con perspectiva correcta.
#[derive(Clone, Copy)]
struct ScreenVertex {
    x: f32,
    y: f32,
    iw: f32,
    v: ClipVertex,
}

fn to_screen(c: &ClipVertex, width: f32, height: f32) -> ScreenVertex {
    let iw = 1.0 / c.clip[3];
    ScreenVertex {
        x: (c.clip[0] * iw + 1.0) * 0.5 * width,
        y: (1.0 - c.clip[1] * iw) * 0.5 * height,
        iw,
        v: *c,
    }
}

/// Recorte contra el plano cercano (z = -w) con Sutherland-Hodgman.
/// Un triángulo puede quedar como 0, 1 o 2 triángulos. Sin esto, al acercarse a un planeta
/// los triángulos que pasan detrás de la cámara se tendrían que botar enteros y se verían hoyos.
fn clip_near(tri: [ClipVertex; 3], out: &mut Vec<ClipVertex>) {
    out.clear();
    let d = |v: &ClipVertex| v.clip[2] + v.clip[3]; // >= 0 adentro
    for i in 0..3 {
        let (a, b) = (&tri[i], &tri[(i + 1) % 3]);
        let (da, db) = (d(a), d(b));
        if da >= 0.0 {
            out.push(*a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            out.push(a.lerp(b, da / (da - db)));
        }
    }
}

pub fn draw(fb: &mut Framebuffer, view_proj: &Mat4, call: &DrawCall, stats: &mut Stats) {
    let mvp = math::mul(view_proj, &call.model);
    let mesh = call.mesh;
    stats.objects += 1;
    stats.vertices += mesh.vertices.len() as u32;
    stats.triangles_in += mesh.indices.len() as u32;

    // Vertex shader (en paralelo si la malla es grande).
    let transformed: Vec<ClipVertex> = if mesh.vertices.len() > 3000 {
        mesh.vertices.par_iter().map(|v| vertex_shader(v, &call.model, &mvp)).collect()
    } else {
        mesh.vertices.iter().map(|v| vertex_shader(v, &call.model, &mvp)).collect()
    };

    let (w, h) = (fb.width as f32, fb.height as f32);
    let mut poly = Vec::with_capacity(4);
    for idx in &mesh.indices {
        let tri = idx.map(|i| transformed[i as usize]);
        let codes = tri.map(|v| v.outcode());
        // Los tres vértices afuera del mismo plano: el triángulo no se ve.
        if codes[0] & codes[1] & codes[2] != 0 {
            stats.triangles_frustum += 1;
            continue;
        }
        let needs_clip = (codes[0] | codes[1] | codes[2]) & 0b1_0000 != 0;
        if needs_clip {
            clip_near(tri, &mut poly);
        } else {
            poly.clear();
            poly.extend_from_slice(&tri);
        }
        if poly.len() < 3 {
            stats.triangles_frustum += 1;
            continue;
        }
        let s: Vec<ScreenVertex> = poly.iter().map(|c| to_screen(c, w, h)).collect();
        for i in 1..s.len() - 1 {
            if rasterize(fb, &s[0], &s[i], &s[i + 1], call) {
                stats.triangles_drawn += 1;
            } else {
                stats.triangles_back += 1;
            }
        }
    }
}

/// Rasteriza un triángulo. Devuelve false si se descartó por estar de espaldas.
fn rasterize(fb: &mut Framebuffer, a: &ScreenVertex, b: &ScreenVertex, c: &ScreenVertex, call: &DrawCall) -> bool {
    // Función de arista: (px - ax)(by - ay) - (py - ay)(bx - ax)
    let area = (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x);
    // Con la y de la pantalla hacia abajo, los triángulos de frente (antihorario) dan área positiva.
    if area <= 0.0 && !call.double_sided {
        return false;
    }
    if area.abs() < 1e-9 {
        return true;
    }

    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as i32;
    let max_x = a.x.max(b.x).max(c.x).ceil().min(fb.width as f32 - 1.0) as i32;
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as i32;
    let max_y = a.y.max(b.y).max(c.y).ceil().min(fb.height as f32 - 1.0) as i32;
    if min_x > max_x || min_y > max_y {
        return true;
    }

    let inv_area = 1.0 / area;
    // Cada función de arista es lineal en x e y: E(x, y) = E0 + dx * x + dy * y.
    // Se evalúa al inicio de cada fila y se le suma `dx` por cada píxel.
    let edge_setup = |p: &ScreenVertex, q: &ScreenVertex| {
        let dx = q.y - p.y;
        let dy = -(q.x - p.x);
        let e0 = -p.x * dx - p.y * dy;
        (e0 * inv_area, dx * inv_area, dy * inv_area)
    };
    let (e0a, e0dx, e0dy) = edge_setup(b, c); // peso del vértice a
    let (e1a, e1dx, e1dy) = edge_setup(c, a); // peso de b
    let (e2a, e2dx, e2dy) = edge_setup(a, b); // peso de c

    for y in min_y..=max_y {
        let py = y as f32 + 0.5;
        let px0 = min_x as f32 + 0.5;
        let mut w0 = e0a + e0dx * px0 + e0dy * py;
        let mut w1 = e1a + e1dx * px0 + e1dy * py;
        let mut w2 = e2a + e2dx * px0 + e2dy * py;
        let row = y as usize * fb.width;
        for x in min_x..=max_x {
            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let index = row + x as usize;
                // 1/w sí se puede interpolar linealmente en pantalla.
                let iw = w0 * a.iw + w1 * b.iw + w2 * c.iw;
                if iw > fb.depth[index] {
                    // Pesos con perspectiva correcta.
                    let k = 1.0 / iw;
                    let (p0, p1, p2) = (w0 * a.iw * k, w1 * b.iw * k, w2 * c.iw * k);
                    let lerp = |va: Vector3, vb: Vector3, vc: Vector3| va * p0 + vb * p1 + vc * p2;
                    let obj = lerp(a.v.obj, b.v.obj, c.v.obj);
                    if call.alpha.is_none_or(|f| f(obj)) {
                        fb.depth[index] = iw;
                        fb.gbuf[index] = GSample {
                            world: lerp(a.v.world, b.v.world, c.v.world),
                            normal: lerp(a.v.normal, b.v.normal, c.v.normal),
                            obj,
                            color: lerp(a.v.color, b.v.color, c.v.color),
                            emit: a.v.emit * p0 + b.v.emit * p1 + c.v.emit * p2,
                            id: call.id,
                        };
                    }
                }
            }
            w0 += e0dx;
            w1 += e1dx;
            w2 += e2dx;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry;

    fn camera() -> Mat4 {
        let view = math::look_at(Vector3::new(0.0, 0.0, 4.0), Vector3::zero(), Vector3::new(0.0, 1.0, 0.0));
        math::mul(&math::perspective(1.0, 1.0, 0.05, 100.0), &view)
    }

    fn call(mesh: &Mesh, model: Mat4) -> DrawCall<'_> {
        DrawCall { mesh, model, id: 1, double_sided: false, alpha: None }
    }

    #[test]
    fn la_mitad_de_la_esfera_esta_de_espaldas() {
        let mesh = geometry::icosphere(3);
        let mut fb = Framebuffer::new(64, 64);
        let mut st = Stats::default();
        draw(&mut fb, &camera(), &call(&mesh, math::identity()), &mut st);
        let half = mesh.indices.len() as f32 / 2.0;
        assert!((st.triangles_back as f32 - half).abs() < half * 0.15);
        assert!(st.triangles_drawn > 0);
    }
}
