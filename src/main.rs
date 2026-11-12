mod culling;
mod framebuffer;
mod geometry;
mod math;
mod mesh;
mod obj;
mod pipeline;

use framebuffer::Framebuffer;
use pipeline::{DrawCall, Stats};
use raylib::prelude::*;
use rayon::prelude::*;

const WIDTH: i32 = 1280;
const HEIGHT: i32 = 720;
const SCALE: usize = 2;

fn render(fb: &mut Framebuffer, sphere: &mesh::Mesh, ship: &mesh::Mesh, t: f32) -> Stats {
    fb.clear();
    let mut stats = Stats::default();
    let eye = Vector3::new(0.0, 1.5, 6.0);
    let view = math::look_at(eye, Vector3::zero(), Vector3::new(0.0, 1.0, 0.0));
    let proj = math::perspective(60f32.to_radians(), fb.width as f32 / fb.height as f32, 0.05, 2000.0);
    let vp = math::mul(&proj, &view);
    let planet = DrawCall {
        mesh: sphere,
        model: math::model_matrix(Vector3::new(-1.5, 0.0, 0.0), 1.5, 0.3, t * 0.5),
        id: 1,
        double_sided: false,
        alpha: None,
    };
    pipeline::draw(fb, &vp, &planet, &mut stats);
    let ship_call = DrawCall {
        mesh: ship,
        model: math::mul(&math::translation(Vector3::new(2.0, 0.0, 1.0)), &math::mul(&math::rotation_y(t), &math::scale(0.3))),
        id: 2,
        double_sided: false,
        alpha: None,
    };
    pipeline::draw(fb, &vp, &ship_call, &mut stats);

    let light = Vector3::new(1.0, 0.6, 0.8).normalized();
    fb.color.par_iter_mut().zip(fb.gbuf.par_iter()).for_each(|(c, s)| {
        *c = if s.id == 0 {
            Vector3::new(0.01, 0.01, 0.03)
        } else {
            let n = s.normal.normalized();
            s.color * (0.05 + n.dot(light).max(0.0))
        };
    });
    stats
}

fn main() {
    let sphere = geometry::icosphere(4);
    let ship = obj::load("assets/nave.obj").unwrap_or_else(|e| panic!("{e}"));
    let mut fb = Framebuffer::new(WIDTH as usize / SCALE, HEIGHT as usize / SCALE);

    if std::env::args().any(|a| a == "shot") {
        render(&mut fb, &sphere, &ship, 0.7);
        fb.save_png("screenshots/prueba.png");
        return;
    }

    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("Proyecto 3 - Space Travel")
        .build();
    rl.set_target_fps(60);
    let img = Image::gen_image_color(fb.width as i32, fb.height as i32, Color::BLACK);
    let mut texture = rl.load_texture_from_image(&thread, &img).expect("textura");
    let mut rgba = Vec::new();

    while !rl.window_should_close() {
        let t = rl.get_time() as f32;
        let stats = render(&mut fb, &sphere, &ship, t);
        fb.to_rgba(&mut rgba);
        texture.update_texture(&rgba).expect("update texture");
        let fps = rl.get_fps();
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture_ex(&texture, Vector2::zero(), 0.0, SCALE as f32, Color::WHITE);
        d.draw_text(&format!("{fps} FPS  triangulos: {}", stats.triangles_drawn), 12, 10, 20, Color::RAYWHITE);
    }
}
