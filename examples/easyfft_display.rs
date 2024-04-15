#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use std::any::Any;
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;

use bounded_vec_deque::BoundedVecDeque;

use piston_window::{EventLoop, PistonWindow, WindowSettings};

use plotters::chart::{ChartBuilder, LabelAreaPosition};
use plotters::series::LineSeries;
use plotters::prelude::{Color, IntoDrawingArea, IntoSegmentedCoord, RED, WHITE};
use plotters_piston_eeg::{draw_piston_window, PistonBackend};


use easyfft::prelude::*;

mod frequency_reader;

/*Volume ideal: 40%*/
const FREQ_DIVISOR: i32 = 8;  // 8
const LENGTH:usize = 4096*(FREQ_DIVISOR as usize);
const FPS: u32 = 60;
const FREQ_QUANTITY: i32 = 40*FREQ_DIVISOR;
static mut Y_MAX: i32 = 0;
static mut NORM: f32 = 1.0;

fn main() {
    let (tx, rx) = mpsc::sync_channel(LENGTH);

    thread::spawn(move || {
        frequency_reader::read_port(tx);
    });

    display(&rx);
}

fn display(rx: &Receiver<isize>){
    let mut samples = BoundedVecDeque::with_capacity(LENGTH, LENGTH);

    let mut window: PistonWindow = WindowSettings::new("Frequências em Tempo Real", [1280, 720])
        .samples(4)
        .build()
        .unwrap();

    // window.set_max_fps(FPS as u64);

    while let Some(_) = draw_piston_window(&mut window, |b| unsafe {
        for value in rx.try_iter().take(LENGTH/FREQ_DIVISOR as usize) {
            samples.push_back(value);
        }

        if samples.len() != LENGTH {
            // println!("\nIndata: ({}) {:?}", samples.len(), samples);
            return Ok(())
        }

        let sample_vec: Vec<f32> = samples.iter().map(|&x| x as f32).collect();
        let fft_values = sample_vec.real_fft();

        let frequency_factor = (10000000.0/FREQ_QUANTITY as f32);
        let freq_mag: Vec<(f32, f32)> =
            fft_values.iter()
                .enumerate()
                .map(move |(k,x)| {
                    let freq = frequency_factor*(k as f32)/((LENGTH) as f32);
                    let mag = x.norm();
                    if freq<1.0 || freq>(FREQ_QUANTITY) as f32{
                        return (freq,0.0);
                    }
                    let val = (freq,mag);
                    Y_MAX = (Y_MAX).max((mag*1.001).round() as i32);
                    val
                }).collect();

        let root = b.into_drawing_area();
        root.fill(&WHITE)?;

        let range_x = (0f32..FREQ_QUANTITY as f32);
        let range_y = (0f32..Y_MAX as f32);
        let x_axis_formatter = Some(&(|&x: &f32| format!("{}",(x/FREQ_DIVISOR as f32) )));

        let mut ctx =
            ChartBuilder::on(&root)
                .margin(40)
                .caption("", ("sans-serif", 40))
                .set_label_area_size(LabelAreaPosition::Left, 60)
                .set_label_area_size(LabelAreaPosition::Bottom, 40)
                // .set_label_area_size(LabelAreaPosition::Right, 60)
                .build_cartesian_2d(
                    range_x, range_y
                )
                .unwrap();

        let mut binding = ctx.configure_mesh();
        let mut mesh_builder =
            binding
                .x_desc("Frequências")
                .y_desc(format!("Magnitude (máxima: {})", Y_MAX))
                .axis_desc_style(("sans-serif", 20))
                .y_label_formatter(&(|&y| format!("{:.1}%",100.0*(y as f32/Y_MAX as f32))));

        let mesh_builder = match x_axis_formatter{
            Some(fmt) => mesh_builder.x_label_formatter(fmt),
            None => mesh_builder
        };

        mesh_builder.draw().unwrap();

        ctx.draw_series(LineSeries::new(freq_mag, &RED)).unwrap();

        Ok(())
    }){}
}
