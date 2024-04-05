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

use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit, FrequencySpectrum};
use spectrum_analyzer::windows::{hann_window, hamming_window, blackman_harris_4term, blackman_harris_7term};
use spectrum_analyzer::scaling::divide_by_N_sqrt;

use splines::{Interpolation, Key, Spline};


mod frequency_reader;


const LENGTH:usize = 4096;
const FPS: u32 = 60;
const FREQ_QUANTITY: i32 = 36;
static mut Y_MAX: i32 = 100_000;
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

    let mut window: PistonWindow = WindowSettings::new("Frequências em Tempo Real", [900, 600])
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

        // let spectrum_window = calculate_window(&window_samples);
        // let spectrum_window = calculate_window_softmax(&window_samples, 1.0);
        let spectrum_window = calculate_window_norm(&window_samples);
        // let spectrum_window = calculate_window_bh7(&window_samples);

        // for (fr, fr_val) in spectrum_window.data().iter() {
        //     println!("{}Hz => {}", fr, fr_val)
        // }

        let root = b.into_drawing_area();
        root.fill(&WHITE)?;

        /* gráfico de comum x:i32
        let range_x = (0..FREQ_QUANTITY);
        let range_y = (0f32..Y_MAX as f32);
        // */

        // /* gráfico de comum x:f32
        let range_x = (0f32..FREQ_QUANTITY as f32);
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
                .set_label_area_size(LabelAreaPosition::Right, 60)
                .build_cartesian_2d(
                    range_x, range_y
                )
            .unwrap();

        ctx.configure_mesh()
            .x_desc("Frequências")
            .y_desc("Magnitude")
            .axis_desc_style(("sans-serif", 20))
            .draw().unwrap();

        draw_curve(ctx, spectrum_window);
        // draw_interpolated_curve(ctx, spectrum_window);
        // draw_bars(ctx, spectrum_window);
        // draw_interpolated_bars(ctx, spectrum_window);

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

fn draw_bars(mut ctx: ChartContext<PistonBackend, Cartesian2d<SegmentedCoord<RangedCoordi32>, RangedCoordi32>>, spectrum_window: FrequencySpectrum ) {
    ctx.draw_series(
        spectrum_window.data().iter()
            .map(|(x, y)|
                (x.val() as f32, y.val() as f32))
            .map(|(x, y)| {
                let x_0 = SegmentValue::Exact(x as i32);
                let x_1 = SegmentValue::Exact((x + 1.0) as i32);
                let y = y as i32;
                let mut bar = Rectangle::new([(x_0, 0), (x_1, y)], RED.filled());
                bar.set_margin(0, 0, 1, 1);
                bar
            })
    )
        .unwrap();
}

fn draw_interpolated_bars(mut ctx: ChartContext<PistonBackend, Cartesian2d<SegmentedCoord<RangedCoordi32>, RangedCoordi32>>, spectrum_window: FrequencySpectrum) {
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
    Y_MAX = 250;
    let fft_window = blackman_harris_7term(&window_samples);

    samples_fft_to_spectrum(
        &fft_window,
        (LENGTH as f32 * 0.845).round() as u32,
        FrequencyLimit::Range(1.0, FREQ_QUANTITY as f32),
        Some(&|val, info| {
            // println!("{}", val);
            val*1800000.0
        }),
    ).unwrap()
}

unsafe fn calculate_window_norm(window_samples: &Vec<f32>) -> FrequencySpectrum{
    Y_MAX = 100;
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
            // println!("{}", (val/NORM));
            (10.0*val/NORM)
        }
        ),
    ).unwrap()
}

unsafe fn calculate_window_softmax(window_samples: &Vec<f32>, temperature: f32) -> FrequencySpectrum{
    Y_MAX = 20_000;
    let max = window_samples.iter().fold(0.0, |pivot, &x| if pivot > x {pivot} else {x});

    let normalized_window: Vec<f32> =  window_samples.iter().map(|&x| ((x/max)/temperature).exp() ).collect();
    NORM = normalized_window.iter()
        .fold(0.0, |sum, num| sum + num);
    // println!("{}", NORM);

    let fft_window = hamming_window(&normalized_window);

    samples_fft_to_spectrum(
        &fft_window,
        (LENGTH as f32 * 0.845).round() as u32,
        FrequencyLimit::Range(1.0, FREQ_QUANTITY as f32),
        Some(&move |val, info| ((val - info.min) / NORM)),
    ).unwrap()
}

unsafe fn calculate_window(window_samples: &Vec<f32>) -> FrequencySpectrum{
    Y_MAX = 30_000;
    let fft_window = hann_window(&window_samples);

    samples_fft_to_spectrum(
        &fft_window,
        (LENGTH as f32 * 0.845).round() as u32,
        // (LENGTH-1) as u32,
        FrequencyLimit::Range(1.0, FREQ_QUANTITY as f32),
        // // optional scale
        None
        // Some(&divide_by_N_sqrt)
        // Some(&|val, info| val - info.min),
    ).unwrap()
}

fn interpolate_values_set(range: Range<i32>, points: &[(f64, f64)]) -> Vec<(i32, f64)> {
    // Criar um vetor de chaves para a interpolação spline
    let keys: Vec<Key<f64, f64>> =
        points.iter().map(|&(x, y)|
            Key::new(x, y, Interpolation::Cosine)).collect();

    let spline = Spline::from_vec(keys);

    // Interpolar os valores para cada x_new no intervalo fornecido
    let interpolated_values: Vec<(i32, f64)> = range
        // .map(|x_new| (x_new, spline.sample(x_new as f64).unwrap_or(0.0)))
        .map(|x_new| (x_new, spline.clamped_sample(x_new as f64).unwrap_or(0.0)))
        .collect();

    interpolated_values
}