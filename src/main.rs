mod autopilot;
mod background;
mod camera;
mod capture;
mod culling;
mod decor;
mod framebuffer;
mod game;
mod geometry;
mod local_orbit;
mod math;
mod mesh;
mod normal_map;
mod obj;
mod overlay;
mod pipeline;
mod render;
mod rng;
mod serpent;
mod shaders;
mod ship;
mod system;
mod tour;

use framebuffer::Framebuffer;
use game::{Game, Input, Mode};
use noise::OpenSimplex;
use raylib::prelude::*;
use render::{Meshes, Renderer};

const WIDTH: i32 = 1280;
const HEIGHT: i32 = 720;

fn read_input(rl: &RaylibHandle) -> Input {
    use KeyboardKey::*;
    let down = |k| rl.is_key_down(k);
    let pressed = |k| rl.is_key_pressed(k);
    let axis = |pos, neg| (down(pos) as i32 - down(neg) as i32) as f32;
    let mut i = Input::default();
    i.controls.thrust = axis(KEY_W, KEY_S);
    i.controls.yaw = axis(KEY_D, KEY_A);
    i.controls.pitch = axis(KEY_UP, KEY_DOWN);
    i.controls.lift = axis(KEY_E, KEY_Q);
    i.controls.brake = down(KEY_SPACE);
    i.controls.boost = down(KEY_LEFT_SHIFT) || down(KEY_RIGHT_SHIFT);
    i.toggle_local = pressed(KEY_TAB);
    for (k, body) in [(KEY_ONE, 1), (KEY_TWO, 2), (KEY_THREE, 3), (KEY_FOUR, 4), (KEY_FIVE, 5), (KEY_SIX, 6), (KEY_SEVEN, 7)] {
        if pressed(k) {
            i.autopilot = Some(body);
        }
    }
    i.toggle_overview = pressed(KEY_V);
    i.toggle_gravity = pressed(KEY_G);
    i.toggle_pause = pressed(KEY_P);
    i.toggle_culling = pressed(KEY_C);
    i.toggle_lod = pressed(KEY_L);
    i.toggle_normals = pressed(KEY_N);
    i.toggle_orbits = pressed(KEY_O);
    i.toggle_minimap = pressed(KEY_M);
    i.seed_delta = pressed(KEY_RIGHT_BRACKET) as i32 - pressed(KEY_LEFT_BRACKET) as i32;
    let dt = rl.get_frame_time();
    i.time_scale_delta = axis(KEY_EQUAL, KEY_MINUS) * dt;
    i.radius_delta = axis(KEY_ZERO, KEY_NINE) * dt * 0.4;
    i.intensity_delta = axis(KEY_PERIOD, KEY_COMMA) * dt * 0.4;
    if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT) {
        let d = rl.get_mouse_delta();
        i.look = (-d.x * 0.005, d.y * 0.005);
        i.dragging = true;
    }
    i.zoom = rl.get_mouse_wheel_move();
    i
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "capture") {
        capture::run(args.get(i + 1).map(|s| s.as_str()));
        return;
    }
    if args.iter().any(|a| a == "tour") {
        let save = !args.iter().any(|a| a == "--sin-cuadros");
        tour::run(960, 540, save);
        return;
    }
    let noise = OpenSimplex::new(7);
    let meshes = Meshes::build(&noise).unwrap_or_else(|e| panic!("{e}"));
    let renderer = Renderer::new(noise);
    let mut game = Game::new(noise);

    let mut scale = 2usize; // 1 = resolución completa, 2 = mitad, 3 = un tercio
    let mut fb = Framebuffer::new(WIDTH as usize / scale, HEIGHT as usize / scale);

    if std::env::args().any(|a| a == "shot") {
        for _ in 0..30 {
            game.update(&Input::default(), 1.0 / 30.0);
        }
        renderer.render(&mut fb, &meshes, &game.frame());
        fb.save_png("screenshots/prueba.png");
        return;
    }

    let (mut rl, thread) = raylib::init().size(WIDTH, HEIGHT).title("Proyecto 3 - Space Travel").build();
    rl.set_target_fps(60);
    rl.set_exit_key(None);
    let make_texture = |rl: &mut RaylibHandle, fb: &Framebuffer| {
        let img = Image::gen_image_color(fb.width as i32, fb.height as i32, Color::BLACK);
        rl.load_texture_from_image(&thread, &img).expect("textura")
    };
    let mut texture = make_texture(&mut rl, &fb);
    let mut rgba = Vec::new();
    let mut show_help = true;
    let mut show_hud = true;
    let mut saved_msg = 0.0f32; // segundos que queda el aviso de captura guardada

    while !rl.window_should_close() {
        let input = read_input(&rl);
        if rl.is_key_pressed(KeyboardKey::KEY_H) {
            show_help = !show_help;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_F2) {
            show_hud = !show_hud; // para grabar sin textos
        }
        if rl.is_key_pressed(KeyboardKey::KEY_F1) {
            // 1 = completa, 2 = mitad, 3 = un tercio (para computadoras lentas)
            scale = scale % 3 + 1;
            fb = Framebuffer::new(WIDTH as usize / scale, HEIGHT as usize / scale);
            texture = make_texture(&mut rl, &fb);
        }
        game.update(&input, rl.get_frame_time());

        let start = std::time::Instant::now();
        let report = renderer.render(&mut fb, &meshes, &game.frame());
        let render_ms = start.elapsed().as_secs_f32() * 1000.0;
        if rl.is_key_pressed(KeyboardKey::KEY_K) {
            fb.save_png(&format!("screenshots/captura_{:.0}.png", rl.get_time() * 1000.0));
            saved_msg = 2.0;
        }
        saved_msg -= rl.get_frame_time();
        fb.to_rgba(&mut rgba);
        texture.update_texture(&rgba).expect("update texture");

        let fps = rl.get_fps();
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture_ex(&texture, Vector2::zero(), 0.0, scale as f32, Color::WHITE);
        if show_hud {
            draw_hud(&mut d, &game, &report, fps, render_ms, show_help);
        }
        if game.paused {
            let w = d.measure_text("PAUSA", 40);
            d.draw_text("PAUSA", WIDTH / 2 - w / 2, HEIGHT / 2 - 120, 40, Color::new(255, 255, 255, 200));
        }
        if saved_msg > 0.0 {
            d.draw_text("Captura guardada en screenshots/", WIDTH / 2 - 150, HEIGHT - 150, 18, Color::LIGHTGRAY);
        }
    }
}

fn draw_hud(d: &mut RaylibDrawHandle, game: &Game, report: &render::Report, fps: u32, ms: f32, help: bool) {
    let white = Color::RAYWHITE;
    let gray = Color::new(200, 205, 215, 255);
    // Etiquetas: no se ponen encima del HUD ni una encima de otra, y el planeta que
    // ya está muy cerca no la necesita.
    let mut used: Vec<(i32, i32)> = Vec::new();
    for l in &report.labels {
        let (x, y) = ((l.x * WIDTH as f32) as i32, (l.y * HEIGHT as f32) as i32);
        if y < 120 || l.distance < 15.0 || used.iter().any(|&(ux, uy)| (ux - x).abs() < 140 && (uy - y).abs() < 22) {
            continue;
        }
        used.push((x, y));
        let text = format!("{}  {:.0} u", l.name, l.distance.max(0.0));
        let w = d.measure_text(&text, 16);
        d.draw_text(&text, x - w / 2, y - 18, 16, gray);
    }

    // Marcador del planeta destino (o del que se está rodeando).
    if let Some((x, y, visible)) = report.target {
        let (x, y) = (x * WIDTH as f32, y * HEIGHT as f32);
        let gold = Color::new(255, 205, 80, 220);
        if visible {
            d.draw_circle_lines(x as i32, y as i32, 14.0, gold);
        } else {
            // Flechita en el borde que apunta hacia donde está.
            let dir = Vector2::new(x - WIDTH as f32 / 2.0, y - HEIGHT as f32 / 2.0).normalized();
            let tip = Vector2::new(x, y) + dir * 12.0;
            let side = Vector2::new(-dir.y, dir.x) * 8.0;
            d.draw_triangle(tip, Vector2::new(x, y) - side, Vector2::new(x, y) + side, gold);
            d.draw_triangle(tip, Vector2::new(x, y) + side, Vector2::new(x, y) - side, gold);
        }
    }

    let mode = match game.mode {
        Mode::Free if game.autopilot.is_some() => "piloto automatico",
        Mode::Free => "vuelo libre",
        Mode::Local(_) => "modo local (orbita)",
    };
    let sel = game.selected();
    let body = &game.system.bodies[sel];
    let (_, alt) = game.system.nearest(game.ship.pos, game.orbit_time);
    let mut y = 10;
    let mut line = |d: &mut RaylibDrawHandle, s: &str, size: i32, c: Color| {
        d.draw_text(s, 12, y, size, c);
        y += size + 4;
    };
    line(d, &format!("{fps} FPS  ({ms:.1} ms render)"), 20, white);
    line(d, &format!("Modo: {mode}   Velocidad: {:.1} u/s", game.ship.speed()), 18, white);
    line(d, &format!("Cerca de: {}   altura: {:.1} u", body.name, alt.max(0.0)), 18, white);
    if let Some(t) = game.autopilot {
        line(d, &format!("Viajando a {} (cualquier tecla de vuelo lo cancela)", game.system.bodies[t].name), 18, Color::GOLD);
    }
    if let Mode::Local(lo) = game.mode {
        let b = &game.system.bodies[lo.body];
        line(d, &format!("{}: {}", b.name, b.description()), 16, Color::new(150, 220, 255, 255));
    }
    if game.gravity_on {
        line(d, &format!("Gravedad: {:.2} u/s2", game.gravity.length()), 16, gray);
    } else {
        line(d, "Gravedad: apagada", 16, gray);
    }
    if game.collided < 1.0 {
        line(d, "Choque!", 18, Color::ORANGE);
    }

    let s = &report.stats;
    let on = |b: bool| if b { "si" } else { "no" };
    let st = &game.settings;
    let lod_txt: Vec<String> =
        game.system.bodies.iter().zip(&report.lods).map(|(b, l)| format!("{}:{}", &b.name[..3], l)).collect();
    let bottom = [
        format!(
            "Triangulos: {} dibujados, {} fuera de vista, {} de espaldas  (de {})",
            s.triangles_drawn, s.triangles_frustum, s.triangles_back, s.triangles_in
        ),
        format!(
            "Objetos: {} dibujados, {} descartados   Superficie: {} dibujados, {} fuera de vista, {} muy lejos",
            s.objects, s.objects_culled, report.decor_drawn, report.decor_culled, report.decor_tiny
        ),
        format!("LOD: {}", lod_txt.join("  ")),
        format!(
            "[C] descarte: {}  [L] LOD: {}  [N] normales: {}  [G] gravedad: {}  [P] pausa: {}",
            on(st.culling),
            on(st.lod),
            on(st.normal_mapping),
            on(game.gravity_on),
            on(game.paused)
        ),
        format!(
            "{}: [9/0] radio x{:.2}  [,/.] intensidad {:.2}   [-/=] velocidad orbital x{:.2}   [ [/] ] semilla {}",
            body.name, body.radius_scale, body.intensity, game.time_scale, game.decor_seed
        ),
    ];
    for (i, t) in bottom.iter().enumerate() {
        d.draw_text(t, 12, HEIGHT - 22 * (bottom.len() - i) as i32 - 4, 16, gray);
    }

    if help {
        let lines = [
            "CONTROLES   (H oculta esta ayuda)",
            "W / S   acelerar / reversa",
            "A / D   girar     Flechas: subir/bajar la nariz",
            "Q / E   bajar / subir      Espacio: frenar",
            "Shift   turbo",
            "Tab     modo local cerca de un planeta",
            "1-7     piloto automatico (7 = la luna Nix)",
            "V       vista del sistema     Mouse der.: mirar",
            "Rueda   distancia de la camara",
            "O orbitas  M minimapa  F1 resolucion  K captura",
            "F2      ocultar todos los textos",
        ];
        let x = WIDTH - 360;
        let mut y = (HEIGHT as f32 * 0.66) as i32 - 8;
        d.draw_rectangle(x - 10, y - 6, 352, lines.len() as i32 * 20 + 10, Color::new(0, 0, 10, 150));
        for l in lines {
            d.draw_text(l, x, y, 16, gray);
            y += 20;
        }
    }
}
