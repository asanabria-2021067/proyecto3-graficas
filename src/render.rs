//! Arma el cuadro completo: decide qué se dibuja (descarte y nivel de detalle), manda los
//! objetos al pipeline, corre los fragment shaders sobre el G-buffer y agrega el fondo,
//! las órbitas, el brillo del sol y el minimapa.
use crate::background::Background;
use crate::culling::Frustum;
use crate::decor::{self, Instance, Models};
use crate::framebuffer::Framebuffer;
use crate::geometry;
use crate::math::{self, Mat4};
use crate::mesh::Mesh;
use crate::normal_map::NormalMaps;
use crate::overlay;
use crate::pipeline::{self, DrawCall, Stats};
use crate::serpent::Serpent;
use crate::shaders::{self, Globals, Material, ShadeObject, barren, ring};
use crate::system::System;
use noise::OpenSimplex;
use raylib::prelude::*;
use rayon::prelude::*;

pub const MAX_LOD: usize = 5;
pub const FOV_Y: f32 = 60.0;
pub const NEAR: f32 = 0.05;
pub const FAR: f32 = 3000.0;

// ids de objeto en el G-buffer: 1..=6 son los cuerpos (índice + 1).
// Los objetos de la superficie usan un id cada uno desde FIRST_DECOR_ID (así su shader
// sabe en qué planeta están y si ahí es de día).
const ID_RING: u16 = 20;
const ID_SHIP: u16 = 22;
const FIRST_DECOR_ID: usize = 32;

/// Mallas que se generan una sola vez al iniciar.
pub struct Meshes {
    pub sphere: Vec<Mesh>,   // icosfera, niveles 0..=MAX_LOD
    pub deformed: Vec<Mesh>, // la misma icosfera deformada (Kharon)
    pub ring: Mesh,
    pub ship: Mesh,
    pub models: Models,
    pub serpent: Serpent,
}

impl Meshes {
    pub fn build(noise: &OpenSimplex) -> Result<Self, String> {
        let sphere = geometry::icosphere_lods(MAX_LOD as u32);
        let deformed = sphere.iter().map(|m| geometry::deform(m, |d| barren::height(noise, d, 0))).collect();
        Ok(Self {
            sphere,
            deformed,
            ring: geometry::ring(ring::INNER, ring::OUTER, 160),
            ship: crate::obj::load("assets/nave.obj")?,
            models: Models::load()?,
            serpent: Serpent::new(2, noise),
        })
    }
}

#[derive(Clone, Copy)]
pub struct CameraView {
    pub eye: Vector3,
    pub target: Vector3,
    pub up: Vector3,
    pub fov: f32, // apertura vertical en grados
}

impl CameraView {
    pub fn view(&self) -> Mat4 {
        math::look_at(self.eye, self.target, self.up)
    }
}

pub fn projection(width: usize, height: usize, fov: f32) -> Mat4 {
    math::perspective(fov.to_radians(), width as f32 / height as f32, NEAR, FAR)
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub culling: bool,
    pub lod: bool,
    pub normal_mapping: bool,
    pub orbits: bool,
    pub minimap: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { culling: true, lod: true, normal_mapping: true, orbits: true, minimap: true }
    }
}

/// Todo lo que el render necesita saber del estado del juego en este cuadro.
pub struct Frame<'a> {
    pub system: &'a System,
    pub orbit_time: f32,
    pub time: f32,
    pub decor: &'a [Vec<Instance>],
    pub ship_model: Option<Mat4>,
    pub ship_thrust: f32,
    pub ship_pos: Vector3,
    pub ship_forward: Vector3,
    pub trail: &'a [Vector3],
    pub camera: CameraView,
    pub settings: Settings,
    pub highlight: Option<usize>,
}

pub struct Label {
    pub name: &'static str,
    pub x: f32, // en [0, 1] del ancho
    pub y: f32,
    pub distance: f32,
}

#[derive(Default)]
pub struct Report {
    pub stats: Stats,
    pub lods: Vec<usize>,
    pub decor_drawn: u32,
    pub decor_culled: u32,
    pub labels: Vec<Label>,
    /// Dónde está el planeta destino en pantalla (x, y en [0, 1]); si está fuera de la
    /// vista se pega al borde y `false` lo indica.
    pub target: Option<(f32, f32, bool)>,
}

/// Radio en píxeles con el que se ve una esfera de radio `r` a distancia `dist`.
fn screen_radius(r: f32, dist: f32, proj: &Mat4, height: usize) -> f32 {
    r * proj[1][1] * height as f32 * 0.5 / dist.max(1e-3)
}

/// Nivel de detalle según el tamaño en pantalla: lo que se ve chiquito no necesita
/// miles de triángulos.
pub fn lod_for(px: f32) -> usize {
    match px {
        x if x < 6.0 => 1,
        x if x < 25.0 => 2,
        x if x < 70.0 => 3,
        x if x < 180.0 => 4,
        _ => 5,
    }
}

pub struct Renderer {
    pub background: Background,
    pub maps: NormalMaps,
    pub noise: OpenSimplex,
}

impl Renderer {
    pub fn new(noise: OpenSimplex) -> Self {
        Self { background: Background::new(&noise), maps: NormalMaps::generate(&noise), noise }
    }

    pub fn render(&self, fb: &mut Framebuffer, meshes: &Meshes, f: &Frame) -> Report {
        fb.clear();
        let mut report = Report::default();
        let view = f.camera.view();
        let proj = projection(fb.width, fb.height, f.camera.fov);
        let vp = math::mul(&proj, &view);
        let frustum = Frustum::from_matrix(&vp);
        let t = f.orbit_time;
        let mut objects = vec![ShadeObject::new(Material::Flat); FIRST_DECOR_ID];

        // ---------- Cuerpos celestes ----------
        for (i, body) in f.system.bodies.iter().enumerate() {
            let so = body.shade_object(t);
            objects[i + 1] = so;
            let r = body.radius();
            let bound = if body.ring { r * ring::OUTER } else { r * 1.2 };
            let dist = (so.center - f.camera.eye).length();
            let px = screen_radius(r, dist, &proj, fb.height);
            let level = if f.settings.lod { lod_for(px) } else { MAX_LOD };
            report.lods.push(level);

            if f.settings.culling && !frustum.sphere_visible(so.center, bound) {
                report.stats.objects_culled += 1;
                continue;
            }
            let mesh = if body.deformed { &meshes.deformed[level] } else { &meshes.sphere[level] };
            let model = math::model_matrix(so.center, r, so.tilt, so.spin);
            pipeline::draw(
                fb,
                &vp,
                &DrawCall { mesh, model, id: i as u16 + 1, double_sided: false, alpha: None },
                &mut report.stats,
            );

            if body.ring {
                let mut ro = so;
                ro.material = Material::Ring;
                objects[ID_RING as usize] = ro;
                let call = DrawCall { mesh: &meshes.ring, model, id: ID_RING, double_sided: true, alpha: Some(ring::alpha) };
                pipeline::draw(fb, &vp, &call, &mut report.stats);
            }

            // ---------- La serpiente de roca (solo en Tessara) ----------
            if i == meshes.serpent.body && (!f.settings.lod || px > 40.0) {
                let (bc, br) = meshes.serpent.bounds(body, &so);
                if f.settings.culling && !frustum.sphere_visible(bc, br) {
                    report.stats.objects_culled += 1;
                } else {
                    for part in meshes.serpent.parts(body, &so, f.time) {
                        if f.settings.culling && !frustum.sphere_visible(part.center, part.radius) {
                            report.stats.objects_culled += 1;
                            continue;
                        }
                        let mut s = ShadeObject::new(Material::Serpent);
                        s.center = so.center;
                        s.intensity = part.param;
                        objects.push(s);
                        let id = objects.len() as u16 - 1;
                        pipeline::draw(
                            fb,
                            &vp,
                            &DrawCall { mesh: part.mesh, model: part.model, id, double_sided: false, alpha: None },
                            &mut report.stats,
                        );
                    }
                }
            }

            // ---------- Objetos de la superficie ----------
            // Solo cuando el planeta se ve grande (si no, serían puntitos de 1 píxel).
            let instances = &f.decor[i];
            if instances.is_empty() || (f.settings.lod && px < 90.0) {
                continue;
            }
            for inst in instances {
                let mesh = meshes.models.mesh(inst.kind);
                let (model, center, radius) = decor::model_matrix(inst, body, &so, &self.noise, mesh.radius);
                if f.settings.culling && !frustum.sphere_visible(center, radius) {
                    report.decor_culled += 1;
                    report.stats.objects_culled += 1;
                    continue;
                }
                report.decor_drawn += 1;
                if objects.len() >= u16::MAX as usize {
                    continue;
                }
                let mut d = ShadeObject::new(Material::Flat);
                d.center = so.center;
                objects.push(d);
                let id = objects.len() as u16 - 1;
                pipeline::draw(fb, &vp, &DrawCall { mesh, model, id, double_sided: false, alpha: None }, &mut report.stats);
            }
        }

        // ---------- Nave ----------
        if let Some(model) = f.ship_model {
            let mut so = ShadeObject::new(Material::Ship);
            so.intensity = f.ship_thrust;
            objects[ID_SHIP as usize] = so;
            pipeline::draw(
                fb,
                &vp,
                &DrawCall { mesh: &meshes.ship, model, id: ID_SHIP, double_sided: false, alpha: None },
                &mut report.stats,
            );
        }

        // ---------- Fragment shaders (en paralelo, una vez por píxel visible) ----------
        let globals = Globals {
            noise: &self.noise,
            maps: &self.maps,
            time: f.time,
            sun: f.system.bodies[0].center(t),
            camera: f.camera.eye,
            normal_mapping: f.settings.normal_mapping,
        };
        // Para el fondo: dirección de vista de cada píxel = r * x + u * y + f.
        let fwd = (f.camera.target - f.camera.eye).normalized();
        let right = fwd.cross(f.camera.up).normalized();
        let up = right.cross(fwd);
        let (sx, sy) = (1.0 / proj[0][0], 1.0 / proj[1][1]);
        let (w, h) = (fb.width, fb.height);
        let bg = &self.background;
        fb.color.par_chunks_mut(w).zip(fb.gbuf.par_chunks(w)).enumerate().for_each(|(y, (row, grow))| {
            let ndc_y = 1.0 - (y as f32 + 0.5) / h as f32 * 2.0;
            for (x, (c, s)) in row.iter_mut().zip(grow).enumerate() {
                *c = if s.id == 0 {
                    let ndc_x = (x as f32 + 0.5) / w as f32 * 2.0 - 1.0;
                    bg.sample((right * (ndc_x * sx) + up * (ndc_y * sy) + fwd).normalized())
                } else {
                    shaders::fragment_shader(s, &objects[s.id as usize], &globals)
                };
            }
        });

        self.background.draw_stars(fb, &vp);
        overlay::trail(fb, &vp, f.trail);
        if f.settings.orbits {
            overlay::orbits(fb, &vp, f.system, f.highlight);
        }
        let ray = |x: usize, y: usize| {
            let ndc_x = (x as f32 + 0.5) / w as f32 * 2.0 - 1.0;
            let ndc_y = 1.0 - (y as f32 + 0.5) / h as f32 * 2.0;
            (right * (ndc_x * sx) + up * (ndc_y * sy) + fwd).normalized()
        };
        for b in &f.system.bodies {
            let color = match b.material {
                Material::Rocky => Some(Vector3::new(0.35, 0.6, 1.0)),
                Material::Ice => Some(Vector3::new(0.5, 0.85, 1.0)),
                Material::Lava => Some(Vector3::new(1.0, 0.35, 0.1)),
                Material::GasGiant => Some(Vector3::new(0.9, 0.7, 0.5)),
                _ => None,
            };
            if let Some(color) = color {
                let sun = f.system.bodies[0].center(t);
                overlay::atmosphere(fb, &ray, b.center(t), b.radius(), f.camera.eye, sun, color);
            }
        }
        overlay::sun_glow(fb, &vp, &proj, f.system.bodies[0].center(t), f.system.bodies[0].radius(), f.camera.eye);
        if f.settings.minimap {
            overlay::minimap(fb, f.system, t, f.ship_pos, f.ship_forward, f.highlight);
        }

        // ---------- Etiquetas de los planetas (las dibuja raylib encima) ----------
        for body in &f.system.bodies {
            let c = body.center(t);
            let top = c + f.camera.up * (body.radius() * 1.25);
            let p = math::transform(&vp, top, 1.0);
            if p[3] <= NEAR {
                continue;
            }
            let (x, y) = ((p[0] / p[3] + 1.0) * 0.5, (1.0 - p[1] / p[3]) * 0.5);
            if (0.0..1.0).contains(&x) && (0.0..1.0).contains(&y) {
                report.labels.push(Label { name: body.name, x, y, distance: (c - f.ship_pos).length() - body.radius() });
            }
        }
        if let Some(i) = f.highlight {
            let c = math::transform(&vp, f.system.bodies[i].center(t), 1.0);
            let (mut x, mut y) = (c[0] / c[3].abs().max(1e-4), -c[1] / c[3].abs().max(1e-4));
            let visible = c[3] > 0.0 && x.abs() <= 1.0 && y.abs() <= 1.0;
            if !visible {
                // Detrás de la cámara o a un lado: se lleva la dirección al borde.
                if c[3] <= 0.0 {
                    x = -x;
                    y = -y;
                }
                let m = x.abs().max(y.abs()).max(1e-4);
                x = x / m * 0.92;
                y = y / m * 0.88;
            }
            report.target = Some(((x + 1.0) * 0.5, (y + 1.0) * 0.5, visible));
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mas_cerca_mas_detalle() {
        let mut prev = 0;
        for px in [1.0, 10.0, 40.0, 100.0, 300.0] {
            let l = lod_for(px);
            assert!(l >= prev && l <= MAX_LOD);
            prev = l;
        }
        assert_eq!(lod_for(1000.0), MAX_LOD);
    }
}
