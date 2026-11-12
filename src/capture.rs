//! Modo sin ventana para sacar las capturas del README:
//! `cargo run --release -- capture [filtro]` guarda en `docs/`.
use crate::framebuffer::Framebuffer;
use crate::game::{Game, Input, Mode};
use crate::render::{Meshes, Renderer};
use noise::OpenSimplex;
use raylib::prelude::*;

/// Escena de captura: la nave en modo local sobre un planeta.
struct Shot {
    name: &'static str,
    body: usize,
    lat: f32, // grados, posición sobre el planeta (en el mundo, no gira con él)
    lon: f32,
    alt: f32, // en radios del planeta
    heading: f32,
    time: f32,
    distance: f32, // distancia de la cámara a la nave
}

const SHOTS: [Shot; 8] = [
    Shot { name: "ignis", body: 1, lat: 10.0, lon: -40.0, alt: 0.45, heading: 0.0, time: 20.0, distance: 4.5 },
    Shot { name: "tessara", body: 2, lat: 20.0, lon: -60.0, alt: 0.35, heading: 30.0, time: 30.0, distance: 4.5 },
    Shot { name: "kharon", body: 3, lat: -15.0, lon: -30.0, alt: 0.35, heading: 80.0, time: 40.0, distance: 4.2 },
    Shot { name: "glacia", body: 4, lat: 35.0, lon: -50.0, alt: 0.4, heading: 10.0, time: 50.0, distance: 4.5 },
    Shot { name: "orvane", body: 5, lat: 18.0, lon: -40.0, alt: 0.5, heading: 60.0, time: 60.0, distance: 6.0 },
    Shot { name: "mirra", body: 6, lat: 25.0, lon: -35.0, alt: 0.55, heading: 20.0, time: 25.0, distance: 3.4 },
    Shot { name: "nix", body: 7, lat: 15.0, lon: -50.0, alt: 1.2, heading: 70.0, time: 70.0, distance: 3.8 },
    Shot { name: "tessara_superficie", body: 2, lat: 25.0, lon: -70.0, alt: 0.1, heading: 40.0, time: 30.0, distance: 3.2 },
];

/// Coloca la nave sobre el planeta (lat/lon medidos respecto a la dirección del sol).
fn place(game: &mut Game, s: &Shot) {
    game.orbit_time = s.time;
    game.time = s.time;
    let b = &game.system.bodies[s.body];
    let c = b.center(s.time);
    let to_sun = (Vector3::zero() - c).normalized();
    let side = Vector3::new(0.0, 1.0, 0.0).cross(to_sun).normalized();
    let (lat, lon) = (s.lat.to_radians(), s.lon.to_radians());
    let dir = (to_sun * lon.cos() + side * lon.sin()) * lat.cos() + Vector3::new(0.0, lat.sin(), 0.0);
    let r = b.radius();
    game.ship.pos = c + dir * (r * (1.0 + s.alt));
    let east = Vector3::new(0.0, 1.0, 0.0).cross(dir).normalized();
    let north = dir.cross(east);
    let h = s.heading.to_radians();
    game.ship.forward = east * h.cos() + north * h.sin();
    game.ship.up = dir;
    game.ship.vel = b.velocity(s.time, 1.0);
    game.enter_local(s.body);
    game.camera.distance = s.distance;
    game.camera.local = true;
    if let Mode::Local(lo) = game.mode {
        game.camera.altitude = lo.alt;
    }
    game.camera.snap(&game.ship);
}

pub fn run(filter: Option<&str>) {
    if filter == Some("rendimiento") {
        benchmark();
        return;
    }
    let noise = OpenSimplex::new(7);
    let meshes = Meshes::build(&noise).unwrap_or_else(|e| panic!("{e}"));
    let renderer = Renderer::new(noise);
    let mut fb = Framebuffer::new(1280, 720);
    let wanted = |name: &str| filter.is_none_or(|f| name.contains(f));

    for s in SHOTS.iter().filter(|s| wanted(s.name)) {
        let mut game = Game::new(noise);
        place(&mut game, s);
        game.settings.minimap = false;
        // Medio segundo de simulación para que la cámara se acomode.
        for _ in 0..15 {
            game.update(&Input::default(), 1.0 / 30.0);
        }
        let report = renderer.render(&mut fb, &meshes, &game.frame());
        let path = format!("docs/{}.png", s.name);
        fb.save_png(&path);
        let mode = matches!(game.mode, Mode::Local(_));
        println!("{path}: {} triangulos, local={mode}", report.stats.triangles_drawn);
    }

    if wanted("serpiente") {
        // La nave se pone detrás de la serpiente (un poco más al norte) mirando hacia ella.
        let mut game = Game::new(noise);
        let t = 24.0;
        game.orbit_time = t;
        game.time = t;
        let s = &meshes.serpent;
        let b = &game.system.bodies[s.body];
        let so = b.shade_object(t);
        let up = so.to_world(s.dir);
        let (east, _) = crate::decor::tangent_frame(up);
        let side = crate::math::rotate_axis(up, east, -0.5);
        let r = b.radius();
        game.ship.pos = so.center + side * (r * 1.32);
        game.ship.up = side;
        game.ship.forward = crate::math::reject(so.center + up * (r * 1.25) - game.ship.pos, side).normalized();
        game.enter_local(s.body);
        game.camera.distance = 5.5;
        game.camera.local = true;
        game.camera.snap(&game.ship);
        game.settings.minimap = false;
        for _ in 0..10 {
            game.update(&Input::default(), 1.0 / 30.0);
        }
        renderer.render(&mut fb, &meshes, &game.frame());
        fb.save_png("docs/serpiente.png");
        println!("docs/serpiente.png");
    }

    if wanted("sistema") {
        let mut game = Game::new(noise);
        game.orbit_time = 12.0;
        game.camera.overview = true;
        game.camera.snap(&game.ship);
        renderer.render(&mut fb, &meshes, &game.frame());
        fb.save_png("docs/sistema.png");
        println!("docs/sistema.png");
    }
}

/// `cargo run --release -- capture rendimiento`: compara el trabajo del renderizador con y sin
/// descarte y niveles de detalle, y lo guarda en docs/rendimiento.md.
pub fn benchmark() {
    let noise = OpenSimplex::new(7);
    let meshes = Meshes::build(&noise).unwrap_or_else(|e| panic!("{e}"));
    let renderer = Renderer::new(noise);
    let mut fb = Framebuffer::new(640, 360);
    let mut out = String::from(
        "# Rendimiento: descarte y niveles de detalle\n\n\
         Generado con `cargo run --release -- capture rendimiento` (640x360, promedio de 5 cuadros).\n\
         \"Triángulos enviados\" son los que llegan al pipeline; los objetos descartados no mandan ninguno.\n\n",
    );
    let scenes: [(&str, Option<usize>); 3] =
        [("Sobre Tessara (superficie)", Some(5)), ("Cerca de Kharon", Some(2)), ("Inicio, en el espacio", None)];
    for (title, shot) in scenes {
        out.push_str(&format!("## {title}\n\n| Descarte | LOD | Objetos dibujados | Objetos descartados | Triángulos enviados | Triángulos rasterizados | ms por cuadro |\n|---|---|---|---|---|---|---|\n"));
        for (culling, lod) in [(true, true), (false, true), (true, false), (false, false)] {
            let mut game = Game::new(noise);
            if let Some(i) = shot {
                place(&mut game, &SHOTS[i]);
            }
            game.settings.culling = culling;
            game.settings.lod = lod;
            for _ in 0..15 {
                game.update(&Input::default(), 1.0 / 30.0);
            }
            let mut report = renderer.render(&mut fb, &meshes, &game.frame());
            let start = std::time::Instant::now();
            for _ in 0..5 {
                report = renderer.render(&mut fb, &meshes, &game.frame());
            }
            let ms = start.elapsed().as_secs_f32() * 1000.0 / 5.0;
            let s = report.stats;
            let yn = |b: bool| if b { "sí" } else { "no" };
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {:.1} |\n",
                yn(culling),
                yn(lod),
                s.objects,
                s.objects_culled,
                s.triangles_in,
                s.triangles_drawn,
                ms
            ));
        }
        out.push('\n');
    }
    std::fs::write("docs/rendimiento.md", &out).expect("no se pudo escribir docs/rendimiento.md");
    println!("{out}");
}
