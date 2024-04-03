use std::ops::Range;
use std::sync::mpsc;
use bounded_vec_deque::BoundedVecDeque;
use piston_window::{EventLoop, PistonWindow, WindowSettings};
use plotters::chart::{ChartBuilder, ChartContext, LabelAreaPosition};
use plotters::series::LineSeries;
use plotters::element::Rectangle;
use plotters::prelude::{Cartesian2d, Color, IntoDrawingArea, IntoSegmentedCoord, RED, SegmentValue, WHITE};
use plotters_piston_eeg::{draw_piston_window, PistonBackend};
use std::sync::mpsc::Receiver;
use std::thread;

use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};
use spectrum_analyzer::windows::{hann_window};
use spectrum_analyzer::scaling::divide_by_N_sqrt;

mod frequency_reader;


const LENGTH:usize = 4096;
const FPS: u32 = 60;
const FREQ_QUANTITY: i32 = 36;
const Y_MAX: i32 = 50000;


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


    while let Some(_) = draw_piston_window(&mut window, |b| {

        for value in rx.try_iter().take(LENGTH) {
            samples.push_back(value);
        }

        if samples.len() != LENGTH {
            //println!("\nIndata: ({}) {:?}", indata.len(), indata);
            return Ok(())
        }

        let window_samples: Vec<f32> = samples.iter().map(|&x| x as f32).collect();
        // println!("{:?}", samples);
        let fft_window = hann_window(&window_samples);

        let spectrum_window = samples_fft_to_spectrum(
            &fft_window,
            (LENGTH as f32 * 0.845).round() as u32,
            // LENGTH as u32,
            FrequencyLimit::Range(1.0, FREQ_QUANTITY as f32),
            // optional scale
            None
            // Some(&divide_by_N_sqrt)
            // Some(&|val, info| val - info.min),
        ).unwrap();

        // for (fr, fr_val) in spectrum_window.data().iter() {
        //     println!("{}Hz => {}", fr, fr_val)
        // }

        let root = b.into_drawing_area();
        root.fill(&WHITE)?;

        let mut ctx =
            ChartBuilder::on(&root)
                .margin(40)
                .caption("", ("sans-serif", 40))
                .set_label_area_size(LabelAreaPosition::Left, 60)
                .set_label_area_size(LabelAreaPosition::Bottom, 40)
                .set_label_area_size(LabelAreaPosition::Right, 60)
                .build_cartesian_2d(
                    (0..FREQ_QUANTITY), // gráfico de comum
                    // (0..FREQ_QUANTITY).into_segmented(), // gráfico de barras
                    0..Y_MAX
                )
            .unwrap();

        ctx.configure_mesh()
            .x_desc("Frequências")
            .y_desc("Magnitude")
            .axis_desc_style(("sans-serif", 20))
            .draw().unwrap();

        /* Gráfico comum Interpolado----------------------------------------------------- */
        let interp = interpolate_values_set((0..FREQ_QUANTITY),
                                            spectrum_window.data().iter()
                                                .map(|(x, y)|
                                                    (x.val() as f64, y.val() as f64))
                                                .collect::<Vec<_>>().as_slice());

        let curva =
                interp.iter()
                    .cloned()
                    .map(|(x,y)| (x, y as i32))
            ;

        ctx.draw_series(LineSeries::new(curva,&RED)).unwrap();
        /* ------------------------------------------------------------------------------ */

        /* Gráfico comum ---------------------------------------------------------------- */
        // let curva =
        //     spectrum_window.data().iter()
        //             .map(|(x, y)| (x.val(), y.val()))
        //         .map(|(x, y)|
        //             (x as i32, y as i32)
        //         )
        //     ;
        //
        // ctx.draw_series(LineSeries::new(curva,&RED)).unwrap();
        /* ------------------------------------------------------------------------------ */


        /* Gráfico de barras ------------------------------------------------------------ */
        // ctx.draw_series(spectrum_window.data().iter()
        //     .map(|(x, y)| (x.val(), y.val()))
        //     .map(|(x, y)| {
        //         let x0 = SegmentValue::Exact(x.round() as i32);
        //         let x1 = SegmentValue::Exact(x.round() as i32 + 1);
        //         let mut bar = Rectangle::new([(x0, 0), (x1, y as i32)], RED.filled());
        //         bar.set_margin(0, 0, 1, 1);
        //         bar
        //     }))
        //     .unwrap();
        /* ------------------------------------------------------------------------------ */

        Ok(())
    }){}
}

fn rustfft_display(rx: Receiver<isize>) {

    let mut samples = BoundedVecDeque::with_capacity(LENGTH, LENGTH);

    let mut window: PistonWindow = WindowSettings::new("Frequências em Tempo Real", [900, 600])
        .samples(4)
        .build()
        .unwrap();

    window.set_max_fps(FPS as u64);


    while let Some(_) = draw_piston_window(&mut window, |b| {

        for value in rx.try_iter().take(LENGTH) {
            samples.push_back(value);
        }

        if samples.len() != LENGTH {
            //println!("\nIndata: ({}) {:?}", indata.len(), indata);
            return Ok(())
        }

        //TODO
        use rustfft::{FftPlanner, num_complex::{Complex64}};

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(LENGTH);

        let mut buffer =
        samples.iter().map(|&x| Complex64{re: x as f64, im:0.0}).collect::<Vec<_>>();

        fft.process(&mut buffer);
        //---

        let root = b.into_drawing_area();
        root.fill(&WHITE)?;

        let mut ctx =
            ChartBuilder::on(&root)
                .margin(40)
                .caption("FFT", ("sans-serif", 40))
                .set_label_area_size(LabelAreaPosition::Left, 60)
                .set_label_area_size(LabelAreaPosition::Bottom, 40)
                .set_label_area_size(LabelAreaPosition::Right, 60)
                .build_cartesian_2d(
                    (0..FREQ_QUANTITY), // gráfico de comum
                    // (0..FREQ_QUANTITY).into_segmented(), // gráfico de barras
                    0..Y_MAX
                )
                .unwrap();

        ctx.configure_mesh()
            .x_desc("Frequências")
            .y_desc("Magnitude")
            .axis_desc_style(("sans-serif", 20))
            .draw().unwrap();


        /* Gráfico comum ---------------------------------------------------------------- */
        let curva =
            buffer.iter()
                .map(|(x)| (x*x.conj()))
                .map(|x| x.re.sqrt())
                .enumerate()
                .map(|(x, y)|
                    (x as i32, y as i32)
                )
                .map(|(x,y)| {
                    if x==0 || x > FREQ_QUANTITY+1 {
                        (x,0)
                    } else {
                        (x,y)
                    }
                })
            ;

        ctx.draw_series(LineSeries::new(curva,&RED)).unwrap();
        /* ------------------------------------------------------------------------------ */

        Ok(())
    }){}
}

fn dft_display(rx: Receiver<isize>) {

    let mut samples = BoundedVecDeque::with_capacity(LENGTH, LENGTH);

    let mut window: PistonWindow = WindowSettings::new("Frequências em Tempo Real", [900, 600])
        .samples(4)
        .build()
        .unwrap();

    window.set_max_fps(FPS as u64);


    while let Some(_) = draw_piston_window(&mut window, |b| {

        for value in rx.try_iter().take(LENGTH) {
            samples.push_back(value);
        }

        if samples.len() != LENGTH {
            //println!("\nIndata: ({}) {:?}", indata.len(), indata);
            return Ok(())
        }

        //TODO
        use dft::{Operation, Plan, c64};
        let plan = Plan::new(Operation::Forward, LENGTH);
        let mut buffer =
            samples.iter()
                .map(|&x| c64::new(x as f64,0.0)).collect::<Vec<_>>();
        dft::transform(&mut buffer, &plan);
        //---

        let root = b.into_drawing_area();
        root.fill(&WHITE)?;

        let mut ctx =
            ChartBuilder::on(&root)
                .margin(40)
                .caption("FFT", ("sans-serif", 40))
                .set_label_area_size(LabelAreaPosition::Left, 60)
                .set_label_area_size(LabelAreaPosition::Bottom, 40)
                .set_label_area_size(LabelAreaPosition::Right, 60)
                .build_cartesian_2d(
                    (0..FREQ_QUANTITY), // gráfico de comum
                    // (0..FREQ_QUANTITY).into_segmented(), // gráfico de barras
                    0..Y_MAX
                )
                .unwrap();

        ctx.configure_mesh()
            .x_desc("Frequências")
            .y_desc("Magnitude")
            .axis_desc_style(("sans-serif", 20))
            .draw().unwrap();


        /* Gráfico comum ---------------------------------------------------------------- */
        let curva =
            buffer.iter()
                .map(|(x)| (x*x.conj()))
                .map(|x| x.re.sqrt())
                .enumerate()
                .map(|(x, y)|
                    (x as i32, y as i32)
                )
                .map(|(x,y)| {
                    if x==0 || x > FREQ_QUANTITY+1 {
                        (x,0)
                    } else {
                        (x,y)
                    }
                })
            ;

        ctx.draw_series(LineSeries::new(curva,&RED)).unwrap();
        /* ------------------------------------------------------------------------------ */

        Ok(())
    }){}
}


use splines::{Interpolation, Key, Spline};

fn interpolate_values_set(range: Range<i32>, points: &[(f64, f64)]) -> Vec<(i32, f64)> {
    // Criar um vetor de chaves para a interpolação spline
    let keys: Vec<Key<f64, f64>> =
        points.iter().map(|&(x, y)|
            Key::new(x, y, Interpolation::Cosine)).collect();

    // Criar uma spline a partir das chaves
    let spline = Spline::from_vec(keys);

    // Interpolar os valores para cada x_new no intervalo fornecido
    let interpolated_values: Vec<(i32, f64)> = range
        // .map(|x_new| (x_new, spline.sample(x_new as f64).unwrap_or(0.0)))
        .map(|x_new| (x_new, spline.clamped_sample(x_new as f64).unwrap_or(0.0)))
        .collect();

    println!("{:?}\n", interpolated_values);
    interpolated_values
}