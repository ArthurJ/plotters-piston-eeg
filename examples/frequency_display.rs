use std::any::Any;
use std::thread;
use std::ops::Range;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;

use bounded_vec_deque::BoundedVecDeque;

use piston_window::{EventLoop, PistonWindow, WindowSettings};

use plotters::chart::{ChartBuilder, ChartContext, LabelAreaPosition};
use plotters::series::LineSeries;
use plotters::element::Rectangle;
use plotters::prelude::{Cartesian2d, Color, IntoDrawingArea, IntoSegmentedCoord, RED, SegmentValue, WHITE};
use plotters_piston_eeg::{draw_piston_window, PistonBackend};
use plotters::coord::ranged1d::SegmentedCoord;
use plotters::coord::types::{RangedCoordf32, RangedCoordf64, RangedCoordi32};
use rustfft::num_traits::Pow;
use serialport::new;

use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit, FrequencySpectrum};
use spectrum_analyzer::windows::{hann_window, hamming_window, blackman_harris_4term, blackman_harris_7term};
use spectrum_analyzer::scaling::divide_by_N_sqrt;

use splines::{Interpolation, Key, Spline};


mod frequency_reader;


const LENGTH:usize = 4096;
const FPS: u32 = 60;
const FREQ_QUANTITY: i32 = 36;
static mut Y_MAX: i32 = 0;
static mut NORM: f32 = 1.0;

fn main() {
    let (tx, rx) = mpsc::sync_channel(LENGTH);

    thread::spawn(move || {
        frequency_reader::read_port(tx);
    });

    spectrum_display(rx);
}

fn spectrum_display(rx: Receiver<isize>) {

    let mut samples = BoundedVecDeque::with_capacity(LENGTH, LENGTH);

    let mut window: PistonWindow = WindowSettings::new("Frequências em Tempo Real", [1280, 720])
        .samples(4)
        .build()
        .unwrap();

    window.set_max_fps(FPS as u64);


    while let Some(_) = draw_piston_window(&mut window, |b| unsafe {
        for value in rx.try_iter().take(LENGTH) {
            samples.push_back(value);
        }

        if samples.len() != LENGTH {
            //println!("\nIndata: ({}) {:?}", indata.len(), indata);
            return Ok(())
        }

        let window_samples: Vec<f32> = samples.iter().map(|&x| x as f32).collect();
        // println!("{:?}", samples);

        let spectrum_window = calculate_window(&window_samples);
        // let spectrum_window = calculate_window_softmax(&window_samples, 5.0);
        // let spectrum_window = calculate_window_norm(&window_samples);
        // let spectrum_window = calculate_window_bh7(&window_samples);

        // for (fr, fr_val) in spectrum_window.data().iter() {
        //     println!("{}Hz => {}", fr, fr_val)
        // }

        let root = b.into_drawing_area();
        root.fill(&WHITE)?;

        /* curva crua (x:f32)
        let range_x = (0f32..FREQ_QUANTITY as f32);
        let range_y = (0f32..Y_MAX as f32);
        // */

        // /* curva interpolada (x:i32)
        let range_x = (0..FREQ_QUANTITY);
        let range_y = (0f32..Y_MAX as f32);
        // */

        /* gráfico de barras
        let range_x = (0..FREQ_QUANTITY).into_segmented();
        let range_y = (0..Y_MAX);
        // */

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

        ctx.configure_mesh()
            .x_desc("Frequências")
            .y_desc(format!("Magnitude (máxima: {})", Y_MAX))
            .axis_desc_style(("sans-serif", 20))
            .y_label_formatter(&(|&x| format!("{:.1}%",100.0* (x as f32/Y_MAX as f32) )))
            .draw().unwrap();

        // draw_curve(ctx, spectrum_window);
        draw_interpolated_curve(ctx, spectrum_window);
        // draw_histogram(ctx, spectrum_window);

        Ok(())
    }){}
}

fn draw_curve(mut ctx: ChartContext<PistonBackend, Cartesian2d<RangedCoordf32, RangedCoordf32>>, spectrum_window: FrequencySpectrum) {
    let curva =
        spectrum_window.data().iter()
            .map(|(x, y)|
                (x.val() as f32, y.val() as f32))
        ;

    ctx.draw_series(LineSeries::new(curva, &RED)).unwrap();
}

fn draw_histogram(mut ctx: ChartContext<PistonBackend, Cartesian2d<SegmentedCoord<RangedCoordi32>, RangedCoordi32>>, spectrum_window: FrequencySpectrum) {
    let data = interpolate_values_set((0..FREQ_QUANTITY),
                                      spectrum_window.data().iter()
                                          .map(|(x, y)|
                                              (x.val() as f64, y.val() as f64))
                                          .collect::<Vec<_>>().as_slice());

    ctx.draw_series(
        data.iter()
            .map(|(x, y)| {
                let x_0 = SegmentValue::Exact(*x as i32);
                let x_1 = SegmentValue::Exact((*x + 1) as i32);
                let y = *y as i32;
                let mut bar = Rectangle::new([(x_0, 0), (x_1, y)], RED.filled());
                bar.set_margin(0, 0, 1, 1);
                bar
            })
    )
        .unwrap();
}

fn draw_interpolated_curve(mut ctx: ChartContext<PistonBackend, Cartesian2d<RangedCoordi32, RangedCoordf32>>, spectrum_window: FrequencySpectrum) {
    let data = interpolate_values_set((0..FREQ_QUANTITY),
                                        spectrum_window.data().iter()
                                            .map(|(x, y)|
                                                (x.val() as f64, y.val() as f64))
                                            .collect::<Vec<_>>().as_slice());

    let curva =
        data.iter()
            .cloned()
            .map(|(x, y)| (x, y as f32))
        ;

    ctx.draw_series(LineSeries::new(curva, &RED)).unwrap();
}

unsafe fn calculate_window_bh7(window_samples: &Vec<f32>) -> FrequencySpectrum{
    let fft_window = blackman_harris_7term(&window_samples);

    let max = fft_window.iter().fold(0.0, |pivot, &x| if pivot > x {pivot} else {x});
    let mag_factor = (magnitude_adjust_factor(max as f64)+1).max((Y_MAX as f32).log10() as i32);
    // println!("{} {}", max, max_mag);
    // println!("{:?}", window_samples);

    samples_fft_to_spectrum(
        &fft_window,
        (LENGTH as f32 * 0.9).round() as u32,
        // (LENGTH-1) as u32,
        FrequencyLimit::Range(1.0, FREQ_QUANTITY as f32),
        Some(&move |val, info| {
            let scaled_val = (val*10.0_f32.powf(mag_factor as f32 - 0.2));
            // println!("{}", scaled_val);
            Y_MAX = (Y_MAX).max((scaled_val *1.001).round() as i32);
            scaled_val
        }),
    ).unwrap()
}

unsafe fn calculate_window_norm(window_samples: &Vec<f32>) -> FrequencySpectrum{
    let fft_window = hann_window(&window_samples);

    NORM = window_samples.iter().fold(0.0, |sum, &num| sum + num.powf(2.0)).sqrt();
    if NORM < 1.0{
        NORM = 1.0;
    }

    samples_fft_to_spectrum(
        &fft_window,
        (LENGTH as f32 * 0.845).round() as u32,
        FrequencyLimit::Range(1.0, FREQ_QUANTITY as f32),
        Some(&|val, info| {
            let scaled_val = (10.0*val/NORM);
            // println!("{}", (val/NORM));
            Y_MAX = (Y_MAX).max((scaled_val *1.001).round() as i32);
            scaled_val
        }
        ),
    ).unwrap()
}

unsafe fn calculate_window_softmax(window_samples: &Vec<f32>, temperature: f32) -> FrequencySpectrum{
    let max = window_samples.iter().fold(0.0, |pivot, &x| if pivot > x {pivot} else {x});

    let exp_window: Vec<f32> =  window_samples.iter().map(|&x| ((x/max)/temperature).exp() ).collect();
    NORM = exp_window.iter()
        .fold(0.0, |sum, num| sum + num);

    let sf_normalized_window: Vec<f32> = exp_window.iter().map(|&x| x/NORM).collect();

    let fft_window = hann_window(&sf_normalized_window);

    let mean = fft_window.iter().fold(0.0, |pivot, &x| pivot+x) as f64/LENGTH as f64;

    let tz = (magnitude_adjust_factor(mean)+1).max((Y_MAX as f32).log10() as i32);

    samples_fft_to_spectrum(
        &fft_window,
        (LENGTH as f32 * 0.9).round() as u32,
        FrequencyLimit::Range(1.0, FREQ_QUANTITY as f32),
        Some(&move |val, info| {
            let scaled_val = (val*10.0_f32.powf(tz as f32));
            println!("y:{} v:{} max:{} y_max:{}, tz:{}", scaled_val, val, info.max, Y_MAX, tz);
            Y_MAX = (Y_MAX).max((scaled_val *1.001).round() as i32);
            scaled_val
        }),
    ).unwrap()
}

unsafe fn calculate_window(window_samples: &Vec<f32>) -> FrequencySpectrum{
    let fft_window = hann_window(&window_samples);

    let max = fft_window.iter().fold(0.0, |pivot, &x| if pivot > x {pivot} else {x});

    samples_fft_to_spectrum(
        &fft_window,
        (LENGTH as f32 * 0.9).round() as u32,
        // (LENGTH-1) as u32,
        FrequencyLimit::Range(1.0, FREQ_QUANTITY as f32),
        // // optional scale
        // None
        // Some(&divide_by_N_sqrt)
        Some(&|val, info| {
            println!("y:{} v:{} max:{}", Y_MAX, val, info.max);
            Y_MAX = (Y_MAX).max((val *1.001).round() as i32);
            val
        }),
    ).unwrap()
}

fn interpolate_values_set(range: Range<i32>, points: &[(f64, f64)]) -> Vec<(i32, f64)> {
    // Criar um vetor de chaves para a interpolação spline
    let keys: Vec<Key<f64, f64>> =
        points.iter().map(|&(x, y)|
            Key::new(x, y, Interpolation::CatmullRom)).collect();

    let spline = Spline::from_vec(keys);

    // Interpolar os valores para cada x_new no intervalo fornecido
    let interpolated_values: Vec<(i32, f64)> = range
        // .map(|x_new| (x_new, spline.sample(x_new as f64).unwrap_or(0.0)))
        .map(|x_new| (x_new, spline.clamped_sample(x_new as f64).unwrap_or(0.0)))
        .collect();

    interpolated_values
}

fn magnitude_adjust_factor(num: f64) -> i32 {
    if num>1.0{
        return 1+num.log10() as i32;
    }
    let mut tmp = num;
    let mut count = 0;

    while (tmp * 10.0).trunc() < 1.0 {
        tmp *= 10.0;
        count += 1;
    }

    count
}