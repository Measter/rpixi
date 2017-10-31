extern crate num;
use num::complex::Complex64;

extern crate piston_window;
use piston_window::*;

extern crate structopt;
#[macro_use]
extern crate structopt_derive;
use structopt::StructOpt;

extern crate image;
use image::{LumaA, Rgba};

extern crate imageproc;
use imageproc::drawing::draw_text_mut;

extern crate rusttype;
use rusttype::{FontCollection, Scale};

extern crate rayon;
use rayon::prelude::*;

extern crate itertools;
use itertools::Itertools;

use std::sync::{Arc, Mutex};
use std::ops::DerefMut;

type Pic<T> = image::ImageBuffer<image::LumaA<T>, std::vec::Vec<T>>;
type SharedData<'a> = Arc<Mutex<(&'a mut Pic<u16>, &'a mut State)>>;

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

    #[structopt(short="f", long="factor", help="Exponent used to determine brightness curve", default_value = "50.0")]
    factor: f64,
    
    #[structopt(short="z", long="zoom", default_value = "350")]
    zoom: u32,

    #[structopt(short="d", long="delta", help="Steps between each coordinate", default_value = "0.05")]
    delta: f64,

    #[structopt(short="l", long="loops", help="Number of iterations for each coordinate", default_value = "200")]
    loop_limit: u32,

    #[structopt(long="re", help="Real offset", default_value = "0.0")]
    off_real: f64,

    #[structopt(long="im", help="Imaginary offset", default_value = "0.0")]
    off_imaginary: f64,
}

#[derive(Debug, Copy, Clone)]
struct State {
    counter: usize,
    max: usize,
    just_finished: bool,
    render_count: u8,
}

fn get_rgba(hue: f64, sat: f64, val: f64) -> Rgba<u8> {
    let hi = (hue/60.0).floor() % 6.0;
    let f = (hue/60.0) - (hue/60.0).floor();
    let p = val * (1.0 - sat);
    let q = val * (1.0 - f * sat);
    let t = val * (1.0 - (1.0-f) * sat);

    match hi as u8 {
        0 => Rgba([(255.0 * val) as u8, (255.0 * t) as u8,   (255.0 * p) as u8, 255]),
        1 => Rgba([(255.0 * q) as u8,   (255.0 * val) as u8, (255.0 * p) as u8, 255]),
        2 => Rgba([(255.0 * p) as u8,   (255.0 * val) as u8, (255.0 * t) as u8, 255]),
        3 => Rgba([(255.0 * p) as u8,   (255.0 * q) as u8,   (255.0 * val) as u8, 255]),
        4 => Rgba([(255.0 * t) as u8,   (255.0 * p) as u8,   (255.0 * val) as u8, 255]),
        5 => Rgba([(255.0 * val) as u8, (255.0 * p) as u8,   (255.0 * q) as u8, 255]),
        _ => Rgba([0,0,0,255])
    }
}


fn mandelbrot(z: Complex64, x: f64, y: f64, cfg: &Config) -> Complex64 {
    let c = Complex64::new(x, y);
    z.powf(cfg.power) + c
}

fn draw_point(image: &mut Pic<u16>, z: Complex64, cfg: &Config) {
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

    let pixel = image.get_pixel_mut(pos_x as u32, pos_y as u32);
    pixel[0] = pixel[0].saturating_add(1);
}

fn u16_to_u8(from: &Pic<u16>, to: &mut image::ImageBuffer<Rgba<u8>, Vec<u8>>, factor: f64) {
    for ((_,_,p), (_,_,p2)) in from.enumerate_pixels().zip(to.enumerate_pixels_mut()) {
        let p = p[0] as f64;
        let val = 1.0 - ((-p)/factor).exp();
        let c = (val * 255.0) as u8;

        *p2 = Rgba([c,c,c,255]);
    }
}

fn iterate_coordinate(x: f64, y: f64, p_data: SharedData, cfg: &Config) {
    let mut z = Complex64::new(cfg.off_real, cfg.off_imaginary);
    // Skip the first value to get rid of the square.
    z = mandelbrot(z, x, y, &cfg);
    let mut points = vec![];

    for _ in 0..cfg.loop_limit {
        z = mandelbrot(z, x, y, &cfg);
        points.push(z);
    }
    
    let mut pic = p_data.lock().expect("Lock failed");
    let &mut (ref mut canvas, ref mut state) = pic.deref_mut();

    for z in points {
        draw_point(canvas, z, &cfg);
    }

    state.counter+=1;
}

fn output_buckets(canvas: &Pic<u16>) {
    let step = 5000;
    let mut buckets = vec![0;14];

    for (_, _, p) in canvas.enumerate_pixels() {
        if p[0] == 0 {
            continue;
        }
        
        buckets[(p[0] / step) as usize] += 1;
    }

    println!("{:#?}", buckets);
}

fn display(cfg: &Config, shared_pic: SharedData) {
    let mut window: PistonWindow = WindowSettings::new("Pixi", (cfg.width, cfg.height))
        .exit_on_esc(true)
        .opengl(OpenGL::V4_5)
        .build()
        .expect("Error creating window");
    
    let mut buffer = image::ImageBuffer::new(cfg.width, cfg.height);
    let mut texture = Texture::from_image( &mut window.factory, &buffer, &TextureSettings::new()).expect("Error creating texture.");

    let font = Vec::from(include_bytes!("SourceSansPro-Light.ttf") as &[u8]);
    let font = FontCollection::from_bytes(font).into_font().unwrap();
    let scale = Scale { x: 12.4 * 2.0, y: 12.4 };

    let cfg_string = format!("{:#?}", cfg);
    let cfg_string: Vec<_> = cfg_string.lines().enumerate().collect();

    let mut factor = cfg.factor;
    let now = std::time::Instant::now();
    let mut force_rerender = false;

    while let Some(e) = window.next() {
        if let Some(_) = e.render_args() {
            let mut pic_data = shared_pic.lock().expect("Lock failed");
            let &mut (ref canvas, ref mut state) = pic_data.deref_mut();

            if force_rerender || (state.render_count == 30 && (!state.just_finished || state.counter != state.max)) {
                u16_to_u8(&canvas, &mut buffer, factor);
                draw_text_mut(&mut buffer, Rgba([255, 255, 255, 255]), 10, 10, scale, &font, &format!("{:.*}% - {}s", 2, (state.counter as f64)/(state.max as f64)*100.0, now.elapsed().as_secs()));

                for &(i, l) in cfg_string.iter() {
                    draw_text_mut(&mut buffer, Rgba([255, 255, 255, 255]), 10, 20 + i as u32*13, scale, &font, l);
                }

                state.just_finished = state.counter == state.max;
                state.render_count = 0;
                force_rerender = false;

                if state.just_finished {
                    //output_buckets(&canvas);
                }
            }

            state.render_count += 1;

            texture.update(&mut window.encoder, &buffer).expect("Error flipping buffer");
            window.draw_2d(&e, |c,g| {
                image(&texture, c.transform, g);
            });
        }

        if let Some(btn) = e.release_args() {
            match btn {
                Button::Mouse(MouseButton::Right) => buffer.save("out.png").unwrap(),
                _ => (),
            }
        }

        if let Some(scroll) = e.mouse_scroll_args() {
            if scroll[1] == -1.0 {
                factor *= 2.0;
                force_rerender = true;
            } else if scroll[1] == 1.0 {
                factor /= 2.0;
                force_rerender = true;
            }
        }
    }

    // Horrid hack to make both threads exit.
    panic!("Exiting.");
}

fn main() {
    let cfg = Config::from_args();

    let mut canvas = image::ImageBuffer::from_pixel(cfg.width, cfg.height, LumaA([0,u16::max_value()]));

    let coords: Vec<_> = (0_u32..).map(|x| -cfg.bounds + x as f64 * cfg.delta).take_while(|&x| x < cfg.bounds).collect();
    let all_coords: Vec<_> = coords.iter().cartesian_product(coords.iter()).collect();

    let mut state = State {
        counter: 0,
        max: all_coords.len(),
        just_finished: false,
        render_count: 0,
    };

    let shared_pic = Arc::new(Mutex::new((&mut canvas, &mut state)));

    rayon::join(
    || all_coords.par_iter().for_each(|&(&y, &x)| iterate_coordinate(x, y, shared_pic.clone(), &cfg) ),
    || display(&cfg, shared_pic.clone()));
}
