use raylib::prelude::*;

const WIDTH: i32 = 1280;
const HEIGHT: i32 = 720;

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("Proyecto 3 - Space Travel")
        .build();
    rl.set_target_fps(60);

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_text("Space Travel", 12, 10, 20, Color::RAYWHITE);
    }
}
