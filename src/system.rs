//! El sistema solar: el sol, seis planetas y una luna, todos en el plano eclíptico (y = 0).
//! Las órbitas son circulares y las posiciones dependen del tiempo, no de los cuadros.
use crate::shaders::{Material, ShadeObject, barren};
use noise::OpenSimplex;
use raylib::prelude::*;
use std::f32::consts::TAU;

/// Órbita circular: radio, segundos por vuelta y ángulo inicial.
#[derive(Clone, Copy, Debug)]
pub struct Orbit {
    pub radius: f32,
    pub period: f32,
    pub phase: f32,
}

impl Orbit {
    pub fn position(&self, t: f32) -> Vector3 {
        if self.period <= 0.0 {
            return Vector3::zero();
        }
        let a = self.phase + TAU * t / self.period;
        Vector3::new(a.cos() * self.radius, 0.0, a.sin() * self.radius)
    }

    pub fn velocity(&self, t: f32, speed: f32) -> Vector3 {
        if self.period <= 0.0 {
            return Vector3::zero();
        }
        let a = self.phase + TAU * t / self.period;
        Vector3::new(-a.sin(), 0.0, a.cos()) * (TAU / self.period * speed * self.radius)
    }
}

pub struct Body {
    pub name: &'static str,
    pub material: Material,
    pub base_radius: f32,
    pub radius_scale: f32, // se puede cambiar en vivo
    pub orbit_radius: f32,
    pub orbit_period: f32, // segundos por vuelta al sol
    pub orbit_phase: f32,
    pub spin_period: f32, // segundos por vuelta sobre su eje
    pub tilt: f32,
    pub gravity: f32,   // aceleración en la superficie
    pub intensity: f32, // parámetro principal del shader
    pub deformed: bool,
    pub ring: bool,
    pub color: Vector3, // para el minimapa y la línea de la órbita
    /// Si es una luna: la órbita del planeta al que le da vueltas.
    pub parent: Option<Orbit>,
    pub terrain_seed: i32, // para los cuerpos deformados
}

impl Body {
    pub fn radius(&self) -> f32 {
        self.base_radius * self.radius_scale
    }

    pub fn orbit(&self) -> Orbit {
        Orbit { radius: self.orbit_radius, period: self.orbit_period, phase: self.orbit_phase }
    }

    /// Centro de la órbita: el sol, o el planeta si es una luna.
    pub fn orbit_center(&self, t: f32) -> Vector3 {
        self.parent.map_or(Vector3::zero(), |p| p.position(t))
    }

    pub fn center(&self, t: f32) -> Vector3 {
        self.orbit_center(t) + self.orbit().position(t)
    }

    /// Derivada de `center`: velocidad del cuerpo en su órbita (`speed` = multiplicador del tiempo).
    /// Una luna suma la velocidad de su planeta.
    pub fn velocity(&self, t: f32, speed: f32) -> Vector3 {
        self.parent.map_or(Vector3::zero(), |p| p.velocity(t, speed)) + self.orbit().velocity(t, speed)
    }

    pub fn spin(&self, t: f32) -> f32 {
        TAU * t / self.spin_period
    }

    pub fn shade_object(&self, t: f32) -> ShadeObject {
        ShadeObject {
            material: self.material,
            center: self.center(t),
            radius: self.radius(),
            spin: self.spin(t),
            tilt: self.tilt,
            deformed: self.deformed,
            intensity: self.intensity,
        }
    }

    /// Altura de la superficie en una dirección del objeto (1.0 = esfera perfecta).
    pub fn height(&self, noise: &OpenSimplex, dir_obj: Vector3) -> f32 {
        if self.deformed { barren::height(noise, dir_obj, self.terrain_seed) } else { 1.0 }
    }

    /// Distancia del centro a la superficie en una dirección del mundo.
    pub fn surface_radius(&self, noise: &OpenSimplex, dir_world: Vector3, t: f32) -> f32 {
        let o = self.shade_object(t);
        self.radius() * self.height(noise, o.to_object(dir_world).normalized())
    }

    /// Masa * G, a partir de la gravedad en la superficie: g = GM / R².
    /// Una línea que describe el cuerpo (se muestra en el HUD en modo local).
    pub fn description(&self) -> &'static str {
        match self.name {
            "Solmira" => "Estrella: plasma con ruido 4D, granulacion y manchas solares",
            "Ignis" => "Lava: placas de Voronoi y lava animada por las grietas",
            "Tessara" => "Rocoso: oceanos, nubes con sombra, ciudades y la serpiente de roca",
            "Kharon" => "Sin atmosfera: geometria deformada con ruido y crateres",
            "Glacia" => "Hielo: mapa de normales agrietado y aurora en los polos",
            "Orvane" => "Gigante gaseoso: bandas que fluyen, tormenta y anillo",
            "Mirra" => "Miniatura: luz en tres tonos, contorno y camino en espiral",
            "Nix" => "Luna de Tessara: otro terreno con crateres",
            _ => "",
        }
    }

    pub fn gm(&self) -> f32 {
        self.gravity * self.radius() * self.radius()
    }
}

pub struct System {
    pub bodies: Vec<Body>, // el 0 es el sol
}

impl System {
    pub fn new() -> Self {
        let body = |name, material, radius, orbit, period, phase: f32, spin, tilt: f32, gravity, color: (f32, f32, f32)| Body {
            name,
            material,
            base_radius: radius,
            radius_scale: 1.0,
            orbit_radius: orbit,
            orbit_period: period,
            orbit_phase: phase.to_radians(),
            spin_period: spin,
            tilt: tilt.to_radians(),
            gravity,
            intensity: 0.5,
            deformed: false,
            ring: false,
            color: Vector3::new(color.0, color.1, color.2),
            parent: None,
            terrain_seed: 0,
        };
        let mut bodies = vec![
            body("Solmira", Material::Star, 14.0, 0.0, 0.0, 0.0, 90.0, 7.0, 14.0, (1.0, 0.75, 0.3)),
            body("Ignis", Material::Lava, 3.6, 46.0, 80.0, 30.0, 26.0, 12.0, 3.0, (1.0, 0.35, 0.1)),
            body("Tessara", Material::Rocky, 6.0, 78.0, 130.0, 150.0, 40.0, 22.0, 4.0, (0.3, 0.6, 1.0)),
            body("Kharon", Material::Barren, 4.4, 108.0, 170.0, 260.0, 55.0, 8.0, 2.5, (0.7, 0.6, 0.75)),
            body("Glacia", Material::Ice, 5.0, 142.0, 220.0, 20.0, 34.0, 28.0, 3.0, (0.6, 0.9, 1.0)),
            body("Orvane", Material::GasGiant, 11.0, 192.0, 300.0, 200.0, 22.0, 15.0, 5.0, (0.9, 0.65, 0.45)),
            body("Mirra", Material::Toon, 2.4, 62.0, 105.0, 250.0, 16.0, 18.0, 2.0, (0.4, 0.85, 0.7)),
            body("Nix", Material::Barren, 1.2, 15.0, 45.0, 90.0, 45.0, 4.0, 1.2, (0.75, 0.75, 0.8)),
        ];
        // Nix es la luna de Tessara: tiene su propio terreno con cráteres.
        bodies[7].parent = Some(bodies[2].orbit());
        bodies[7].deformed = true;
        bodies[7].terrain_seed = 1;
        bodies[7].intensity = 0.3;
        bodies[0].intensity = 0.5;
        bodies[2].intensity = 0.48; // cobertura de nubes
        bodies[6].intensity = 0.6; // florecitas de Mirra
        bodies[3].deformed = true;
        bodies[5].ring = true;
        Self { bodies }
    }

    /// Índice y distancia a la superficie del cuerpo más cercano a `p`.
    pub fn nearest(&self, p: Vector3, t: f32) -> (usize, f32) {
        self.bodies
            .iter()
            .enumerate()
            .map(|(i, b)| (i, (p - b.center(t)).length() - b.radius()))
            .fold((0, f32::MAX), |a, b| if b.1 < a.1 { b } else { a })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todos_en_el_plano_ecliptico() {
        let s = System::new();
        for b in &s.bodies {
            for t in [0.0, 13.0, 200.0] {
                assert_eq!(b.center(t).y, 0.0);
            }
        }
    }

    #[test]
    fn una_vuelta_completa_regresa_al_inicio() {
        let s = System::new();
        for b in s.bodies.iter().skip(1).filter(|b| b.parent.is_none()) {
            let d = (b.center(0.0) - b.center(b.orbit_period)).length();
            assert!(d < 1e-2, "{}", b.name);
        }
    }

    #[test]
    fn la_velocidad_es_la_derivada_de_la_posicion() {
        let s = System::new();
        let b = &s.bodies[2];
        let h = 1e-2;
        let numeric = (b.center(10.0 + h) - b.center(10.0 - h)) * (1.0 / (2.0 * h));
        assert!((numeric - b.velocity(10.0, 1.0)).length() < 1e-2);
    }

    #[test]
    fn las_orbitas_no_se_cruzan() {
        let s = System::new();
        let mut planets: Vec<_> = s.bodies.iter().skip(1).filter(|b| b.parent.is_none()).collect();
        planets.sort_by(|a, b| a.orbit_radius.total_cmp(&b.orbit_radius));
        assert!(planets[0].orbit_radius - planets[0].radius() > s.bodies[0].radius() * 2.0);
        for w in planets.windows(2) {
            let gap = w[1].orbit_radius - w[0].orbit_radius;
            assert!(gap > (w[0].radius() + w[1].radius()) * 1.5, "{} y {}", w[0].name, w[1].name);
        }
    }

    #[test]
    fn el_mas_cercano() {
        let s = System::new();
        let c = s.bodies[4].center(0.0);
        let (i, d) = s.nearest(c + Vector3::new(0.0, 10.0, 0.0), 0.0);
        assert_eq!(i, 4);
        assert!((d - (10.0 - s.bodies[4].radius())).abs() < 1e-3);
    }
}
