extern crate num;
use num::complex::Complex64;

extern crate structopt;
#[macro_use]
extern crate structopt_derive;
use structopt::StructOpt;

extern crate image;
use image::{Rgba, Pixel};

extern crate itertools;
use itertools::Itertools;

extern crate imageproc;
use imageproc::drawing::draw_text_mut;

extern crate rusttype;
use rusttype::{FontCollection, Scale};

extern crate rayon;
use rayon::prelude::*;

extern crate indicatif;
use indicatif::{ProgressBar, ProgressStyle};

use std::sync::{Arc, Mutex};

type Pic<T> = image::ImageBuffer<image::Rgba<T>, std::vec::Vec<T>>;

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
}

fn mandelbrot(z: Complex64, x: f64, y: f64, cfg: &Config) -> Complex64 {
    let c = Complex64::new(x, y);
    z.powf(cfg.power) + c
}

fn draw_point(image: &mut Pic<u16>, z: Complex64, color: Rgba<u16>, cfg: &Config) {
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

fn u16_to_u8(from: &image::ImageBuffer<Rgba<u16>, Vec<u16>>, to: &mut image::ImageBuffer<Rgba<u8>, Vec<u8>> ) {
    for ((_,_,p), (_,_,p2)) in from.enumerate_pixels().zip(to.enumerate_pixels_mut()) {
        p2[3] = (p[3] >> 8) as u8;
        p2[2] = (p[2] >> 8) as u8;
        p2[1] = (p[1] >> 8) as u8;
        p2[0] = (p[0] >> 8) as u8;
    }
}

fn make_frame(cfg: &Config) {
    let mut pic = Pic::from_pixel(cfg.width, cfg.height, Rgba([0,0,0,u16::max_value()]));
    let color = Rgba([u16::max_value(), u16::max_value(), u16::max_value(), (u16::max_value() as f64 * cfg.opacity) as u16]);

    let now = std::time::Instant::now();
    let cfg_string = format!("{:#?}", cfg);
    let cfg_string: Vec<_> = cfg_string.lines().enumerate().collect();

    {
        let shared_pic = Arc::new(Mutex::new(&mut pic));
        
        let coords: Vec<_> = (0_u32..).map(|x| -cfg.bounds + x as f64 * cfg.delta).take_while(|&x| x < cfg.bounds).collect();
        let all_coords: Vec<_> = coords.iter().cartesian_product(coords.iter()).collect();

        let bar = ProgressBar::new(all_coords.len() as u64);
        bar.set_style(ProgressStyle::default_bar()
                      .template("[{elapsed_precise}/{eta_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}"));

        all_coords.par_iter()
            .for_each(|&(&x, &y)| {
                let mut z = Complex64::new(0.0, 0.0);
                // Skip the first value to get rid of the square.
                z = mandelbrot(z, x, y, &cfg)
                let mut points = vec![];

                for _ in 0..cfg.loop_limit {
                    z = mandelbrot(z, x, y, &cfg);
                    points.push(z);
                }
                
                let mut pic = shared_pic.lock().expect("Unlock failed");

                for z in points {
                    draw_point(&mut pic, z, color, &cfg);
                }

                bar.inc(1);
            });
        
        bar.finish();
    }

    let font = Vec::from(include_bytes!("SourceSansPro-Light.ttf") as &[u8]);
    let font = FontCollection::from_bytes(font).into_font().unwrap();
    let scale = Scale { x: 12.4 * 2.0, y: 12.4 };
    let mut buffer = Pic::from_pixel(cfg.width, cfg.height, Rgba([0,0,0,u8::max_value()]));
    u16_to_u8(&pic, &mut buffer);
    draw_text_mut(&mut buffer, Rgba([255, 255, 255, 255]), 10, 10, scale, &font, &format!("{}s", now.elapsed().as_secs()));

    for &(i, l) in cfg_string.iter() {
        draw_text_mut(&mut buffer, Rgba([255, 255, 255, 255]), 10, 20 + i as u32*13, scale, &font, l);
    }
    let _ = buffer.save("out.png").unwrap();
}

fn main() {
    let cfg = Config::from_args();
    make_frame(&cfg);
}
