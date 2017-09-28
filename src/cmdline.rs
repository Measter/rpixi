extern crate num;
use num::complex::Complex64;

extern crate image;
use image::{Rgba, Pixel};

extern crate itertools;
use itertools::Itertools;

extern crate rayon;
use rayon::prelude::*;

use std::sync::{Arc, Mutex};

type Pic = image::ImageBuffer<image::Rgba<u8>, std::vec::Vec<u8>>;

#[derive(Debug)]
struct Config {
    width: u32,
    height: u32,
    bounds: f64,
    colour_factor: f64,
    zoom: u32,
    delta: f64,
    loop_limit: u32,
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

fn mandelbrot(z: Complex64, x: f64, y: f64, cfg: &Config) -> (Complex64, Rgba<u8>) {
    let c = Complex64::new(x, y);
    let zn = z.powf(2.0) + c;

    let mut color = get_rgba(cfg.colour_factor * 360.0 * (c.norm() / cfg.bounds).cos(), 1.0, 1.0);
    color[3] = 20;

    (zn, color)
}

fn draw_point(image: &mut Pic, z: Complex64, color: Rgba<u8>, cfg: &Config) {
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

fn make_frame(cfg: &Config) {
    let mut pic = Pic::from_pixel(cfg.width, cfg.height, Rgba([0,0,0,255]));

    {
        let shared_pic = Arc::new(Mutex::new(&mut pic));
        
        let coords: Vec<_> = (0_u32..).map(|x| -cfg.bounds + x as f64 * cfg.delta).take_while(|&x| x < cfg.bounds).collect();
        let all_coords: Vec<_> = coords.iter().cartesian_product(coords.iter()).collect();

        all_coords.par_iter()
            .for_each(move |&(&x, &y)| {
                let mut z = Complex64::new(0.0, 0.0);
                let mut points = vec![];

                for _ in 0..cfg.loop_limit {
                    let (zn, color) = mandelbrot(z, x, y, &cfg);
                    z = zn;
                    points.push((z, color));
                }
                
                let mut pic = shared_pic.lock().expect("Unlock failed");

                for (z, color) in points {
                    draw_point(&mut pic, z, color, &cfg);
                }
            });
    }
    let _ = pic.save("out.png").unwrap();
}

fn main() {
    let cfg = Config {
        width: 1440,
        height: 1440,
        bounds: 0.9,
        colour_factor: 0.7,
        zoom: 700,
        delta: 0.005,
        loop_limit: 200_000,
    };
    use std::time::Instant;
    let now = Instant::now();
    make_frame(&cfg);
    println!("{:?}", now.elapsed());
}
