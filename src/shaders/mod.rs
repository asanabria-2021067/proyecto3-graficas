//! Fragment shaders. Todos reciben lo que dejó el depth test en el G-buffer y devuelven el
//! color final del píxel. No hay texturas de color: todo sale de ruido y de la luz del sol.
pub mod barren;
pub mod flat;
pub mod gas_giant;
pub mod ice;
pub mod lava;
pub mod noise_helpers;
pub mod ring;
pub mod rocky;
pub mod serpent;
pub mod star;
pub mod toon;

use crate::framebuffer::GSample;
use crate::math;
use crate::normal_map::NormalMaps;
use noise::OpenSimplex;
use raylib::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Material {
    Star,
    Rocky,
    GasGiant,
    Lava,
    Ice,
    Barren,
    Ring,
    Toon,    // Mirra, el planeta miniatura
    Serpent, // la serpiente de roca de Tessara
    Flat,    // modelos de la superficie (árboles, rocas, casas...)
    Ship,
}

/// Uniforms de cada objeto: dónde está, cómo está girado y su parámetro principal.
#[derive(Clone, Copy, Debug)]
pub struct ShadeObject {
    pub material: Material,
    pub center: Vector3,
    pub radius: f32,
    pub spin: f32,
    pub tilt: f32,
    pub deformed: bool, // la malla fue deformada: hay que usar su normal real
    pub intensity: f32, // parámetro que se puede mover en vivo (nubes, lava, tormenta...)
}

impl ShadeObject {
    pub fn new(material: Material) -> Self {
        Self { material, center: Vector3::zero(), radius: 1.0, spin: 0.0, tilt: 0.0, deformed: false, intensity: 0.5 }
    }

    /// mundo -> objeto: deshace la inclinación y luego el giro.
    pub fn to_object(self, d: Vector3) -> Vector3 {
        math::rotate_y(math::rotate_z(d, -self.tilt), -self.spin)
    }

    /// objeto -> mundo
    pub fn to_world(self, d: Vector3) -> Vector3 {
        math::rotate_z(math::rotate_y(d, self.spin), self.tilt)
    }
}

/// Uniforms que comparten todos los objetos del cuadro.
pub struct Globals<'a> {
    pub noise: &'a OpenSimplex,
    pub maps: &'a NormalMaps,
    pub time: f32,
    pub sun: Vector3,
    pub camera: Vector3,
    pub normal_mapping: bool,
}

/// Punto de entrada: el render llama esta función por cada píxel visible.
pub fn fragment_shader(s: &GSample, o: &ShadeObject, g: &Globals) -> Vector3 {
    match o.material {
        Material::Star => star::star(s, o, g),
        Material::Rocky => rocky::rocky(s, o, g),
        Material::GasGiant => gas_giant::gas_giant(s, o, g),
        Material::Lava => lava::lava(s, o, g),
        Material::Ice => ice::ice(s, o, g),
        Material::Barren => barren::barren(s, o, g),
        Material::Ring => ring::ring(s, o, g),
        Material::Toon => toon::toon(s, o, g),
        Material::Serpent => serpent::serpent(s, o, g),
        Material::Flat => flat::flat(s, o, g),
        Material::Ship => flat::ship(s, o, g),
    }
}

pub fn mix(a: Vector3, b: Vector3, t: f32) -> Vector3 {
    a + (b - a) * t
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn rgb(r: f32, g: f32, b: f32) -> Vector3 {
    Vector3::new(r, g, b)
}

/// Degradado de colores: `stops` son pares (posición, color) ordenados.
pub fn ramp(stops: &[(f32, Vector3)], t: f32) -> Vector3 {
    if t <= stops[0].0 {
        return stops[0].1;
    }
    for w in stops.windows(2) {
        let (a, b) = (w[0], w[1]);
        if t <= b.0 {
            return mix(a.1, b.1, smoothstep(a.0, b.0, t));
        }
    }
    stops[stops.len() - 1].1
}

/// Datos comunes de los planetas, en espacio del objeto.
pub struct Surface {
    pub p: Vector3, // punto sobre la esfera unitaria (para los patrones)
    pub n: Vector3, // normal
    pub l: Vector3, // dirección hacia el sol
    pub v: Vector3, // dirección hacia la cámara
}

pub fn surface(s: &GSample, o: &ShadeObject, g: &Globals) -> Surface {
    let p = s.obj.normalized();
    // En una esfera perfecta la normal es la misma posición: se ve lisa aunque la malla
    // tenga pocos triángulos. Si la malla está deformada se usa la normal interpolada.
    let n = if o.deformed { o.to_object(s.normal).normalized() } else { p };
    let l = o.to_object((g.sun - s.world).normalized());
    let v = o.to_object((g.camera - s.world).normalized());
    Surface { p, n, l, v }
}
