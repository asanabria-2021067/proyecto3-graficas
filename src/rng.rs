//! Generador de números pseudoaleatorios (PCG32). Con la misma semilla siempre salen los
//! mismos números, así la distribución de objetos sobre los planetas es reproducible.
use raylib::prelude::*;

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut r = Rng { state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(0x1234_5678) };
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Número en [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }

    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f32()
    }

    /// Punto uniforme sobre la esfera unitaria (z uniforme en [-1, 1] y ángulo uniforme).
    pub fn on_sphere(&mut self) -> Vector3 {
        let z = self.range(-1.0, 1.0);
        let a = self.range(0.0, std::f32::consts::TAU);
        let r = (1.0 - z * z).max(0.0).sqrt();
        Vector3::new(r * a.cos(), z, r * a.sin())
    }
}
