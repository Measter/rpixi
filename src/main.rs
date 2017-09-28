extern crate num;
use num::complex::Complex64;

extern crate piston_window;
use piston_window::*;

extern crate structopt;
#[macro_use]
extern crate structopt_derive;
use structopt::StructOpt;

extern crate image;
use image::{Rgba, Pixel};

type Pic = image::ImageBuffer<image::Rgba<u16>, std::vec::Vec<u16>>;

#[derive(Debug, StructOpt)]
struct Config {
    #[structopt(short="w", long="width", help="Window width", default_value = "1280")]
    width: u32,

    #[structopt(short="h", long="height", help="Window height", default_value = "720")]
    height: u32,

    #[structopt(short="b", long="bounds", help="Minimum and maximum value for bounds", default_value = "0.6")]
    bounds: f64,

    #[structopt(short="p", long="power", help="Power to use in the Mandelbrot equation", default_value = "2.0")]
    power: f64,

    #[structopt(short="c", help="Multiplier on the hue", default_value = "0.7")]
    colour_factor: f64,

    #[structopt(short="o", help="Opacity of the drawn pixel", default_value = "0.8")]
    opacity: f64,
    
    #[structopt(short="z", long="zoom", default_value = "350")]
    zoom: u32,

    #[structopt(short="d", long="delta", help="Steps between each coordinate", default_value = "0.05")]
    delta: f64,

    #[structopt(short="l", long="loops", help="Number of iterations for each coordinate", default_value = "200")]
    loop_limit: u32,

    #[structopt(short="s", long="speed", help="Number of iterations per frame", default_value = "150")]
    speed: u32,
}

#[derive(Debug)]
struct State {
    x: f64,
    y: f64,
    loop_count: u32,
    z: Complex64,
    finished: bool,
}


fn get_rgba(hue: f64, sat: f64, val: f64) -> Rgba<u16> {
    let hi = (hue/60.0).floor() % 6.0;
    let f = (hue/60.0) - (hue/60.0).floor();
    let p = val * (1.0 - sat);
    let q = val * (1.0 - f * sat);
    let t = val * (1.0 - (1.0-f) * sat);

    match hi as u16 {
        0 => Rgba([(u16::max_value() as f64 * val) as u16, (u16::max_value() as f64 * t) as u16,   (u16::max_value() as f64 * p) as u16,   u16::max_value()]),
        1 => Rgba([(u16::max_value() as f64 * q) as u16,   (u16::max_value() as f64 * val) as u16, (u16::max_value() as f64 * p) as u16,   u16::max_value()]),
        2 => Rgba([(u16::max_value() as f64 * p) as u16,   (u16::max_value() as f64 * val) as u16, (u16::max_value() as f64 * t) as u16,   u16::max_value()]),
        3 => Rgba([(u16::max_value() as f64 * p) as u16,   (u16::max_value() as f64 * q) as u16,   (u16::max_value() as f64 * val) as u16, u16::max_value()]),
        4 => Rgba([(u16::max_value() as f64 * t) as u16,   (u16::max_value() as f64 * p) as u16,   (u16::max_value() as f64 * val) as u16, u16::max_value()]),
        5 => Rgba([(u16::max_value() as f64 * val) as u16, (u16::max_value() as f64 * p) as u16,   (u16::max_value() as f64 * q) as u16,   u16::max_value()]),
        _ => Rgba([0,0,0,u16::max_value()])
    }
}

fn mandelbrot(z: Complex64, x: f64, y: f64, cfg: &Config) -> (Complex64, Rgba<u16>) {
    let c = Complex64::new(x, y);
    let zn = z.powf(cfg.power) + c;

    let mut color = get_rgba(cfg.colour_factor * 360.0 * (c.norm() / cfg.bounds).cos(), 1.0, 1.0);
    color[3] = (cfg.opacity * u16::max_value() as f64) as u16;

    (zn, color)
}

fn draw_point(image: &mut Pic, z: Complex64, color: Rgba<u16>, cfg: &Config) {
    let pos_x = (cfg.width as f64/2.0) + z.re * cfg.zoom as f64 - 0.5;
    let pos_y = (cfg.height as f64/2.0) - z.im * cfg.zoom as f64 + 0.5;

    if pos_x + 1.0 < 0.0 || pos_y < 0.0 {
        return;
    }

    let pos_x = pos_x as u32;
    let pos_y = pos_y as u32;

    if pos_x >= cfg.width || pos_y >= cfg.height {
        return;
    }

    image.get_pixel_mut(pos_x as u32, pos_y as u32).blend(&color);
}

fn draw_frame(state: &mut State, cfg: &Config, pic: &mut Pic) {
    for _ in 0..cfg.speed {
        let (zn, color) = mandelbrot(state.z, state.x, state.y, &cfg);
        state.z = zn;
        draw_point(pic, state.z, color, &cfg);
        state.loop_count += 1;

        if state.loop_count >= cfg.loop_limit {
            state.loop_count = 0;
            state.z = Complex64::new(0.0, 0.0);

            if state.x < cfg.bounds {
                state.x += cfg.delta;
            } else if state.y < cfg.bounds {
                state.y += cfg.delta;
                state.x = -cfg.bounds;
            }
        }

        if state.x >= cfg.bounds && state.y >= cfg.bounds {
            state.finished = true;
            break;
        }
    }
}

fn u16_to_u8(from: &image::ImageBuffer<Rgba<u16>, Vec<u16>>, to: &mut image::ImageBuffer<Rgba<u8>, Vec<u8>> ) {
    for ((_,_,p), (_,_,p2)) in from.enumerate_pixels().zip(to.enumerate_pixels_mut()) {
        p2[3] = (p[3] >> 8) as u8;
        p2[2] = (p[2] >> 8) as u8;
        p2[1] = (p[1] >> 8) as u8;
        p2[0] = (p[0] >> 8) as u8;
    }
}

fn main() {
    let cfg = Config::from_args();
    
    let mut window: PistonWindow = WindowSettings::new("Pixi", (cfg.width, cfg.height))
        .exit_on_esc(true)
        .opengl(OpenGL::V4_5)
        .build()
        .expect("Error creating window");

    let mut canvas = image::ImageBuffer::from_pixel(cfg.width, cfg.height, Rgba([0,0,0,u16::max_value()]));
    let mut buffer = image::ImageBuffer::new(cfg.width, cfg.height);
    let mut texture = Texture::from_image( &mut window.factory, &buffer, &TextureSettings::new()).expect("Error creating texture.");

    let mut state = State {
        x: -cfg.bounds,
        y: -cfg.bounds,
        loop_count: 0,
        z: Complex64::new(0.0, 0.0),
        finished: false,
    };

    while let Some(e) = window.next() {
        if let Some(_) = e.render_args() {
            if !state.finished {
                draw_frame(&mut state, &cfg, &mut canvas);
                u16_to_u8(&canvas, &mut buffer);
            }
            texture.update(&mut window.encoder, &buffer).expect("Error flipping buffer");
            window.draw_2d(&e, |c,g| {
                image(&texture, c.transform, g);
            });
        }

        if let Some(btn) = e.release_args() {
            match btn {
                Button::Mouse(MouseButton::Left) => {
                    canvas.enumerate_pixels_mut().for_each(|(_,_, p)| *p = Rgba([0,0,0,u16::max_value()]) );
                    state.x = -cfg.bounds;
                    state.y = -cfg.bounds;
                    state.z = Complex64::new(0.0, 0.0);
                    state.loop_count = 0;
                    state.finished = false;
                },
                Button::Mouse(MouseButton::Right) => buffer.save("out.png").unwrap(),
                _ => (),
            }
        }
    }
}
