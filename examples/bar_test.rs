use piston_window::{EventLoop, PistonWindow, WindowSettings};
use plotters_piston_eeg::draw_piston_window;
use std::collections::{BTreeMap as Map};
use plotters::prelude::*;
use std::io::{self, BufRead, BufReader};
use std::iter::FromIterator;
use std::process::exit;
use std::str::FromStr;
use std::string::ToString;
use std::sync::mpsc;
use std::sync::mpsc::{SendError, SyncSender};
use std::thread;
use std::time::Duration;
use serialport::SerialPort;
use plotters::prelude::SegmentValue;

const FPS: u32 = 60;
const PORT_NAME: &str = "/dev/ttyUSB0";
// const PORT_NAME: String = "/dev/ttyACM0".to_string();
const LENGTH: usize = 35;
const BAUD_RATE: u32 = 115200;

fn main() {

    let (tx, rx) = mpsc::sync_channel(LENGTH);

    thread::spawn(move || {
        read_port(tx);
    });


    let mut window: PistonWindow = WindowSettings::new("Frequências em Tempo Real", [900, 600])
        .samples(4)
        .build()
        .unwrap();

    // window.set_max_fps(FPS as u64);

    let mut samples = Map::new();
    while let Some(_) = draw_piston_window(&mut window, |b| {

        for (k, v) in rx.try_iter().take(LENGTH) {
            // println!("{}:{}", k,v);
            samples.insert(k,v);
        }

        let interpolated = interpolate_dictionary(
            &Map::from_iter(samples.iter()
                .map(|(x,y)|
                    (x.clone(),
                     f64::from_str(y).unwrap_or(0.0)))
            )
        );

        // println!("{:?}", samples);

        let root = b.into_drawing_area();
        root.fill(&WHITE)?;

        let mut ctx = ChartBuilder::on(&root)
            .margin(40)
            .caption("FFT", ("sans-serif", 40))
            .set_label_area_size(LabelAreaPosition::Left, 60)
            .set_label_area_size(LabelAreaPosition::Bottom, 40)
            .set_label_area_size(LabelAreaPosition::Right, 60)
            .build_cartesian_2d((0..LENGTH).into_segmented(), 0..3000)
            .unwrap();

        ctx.configure_mesh()
            .x_desc("Frequências")
            .y_desc("Magnitude")
            .axis_desc_style(("sans-serif", 20))
            .draw().unwrap();

        ctx.draw_series(
            interpolated.iter()
                .map(|(x, y)| {
                    let x0 = SegmentValue::Exact(*x as usize);
                    let x1 = SegmentValue::Exact(*x as usize + 1);
                    let mut bar = Rectangle::new([(x0, 0), (x1, *y as i32)], RED.filled());
                    bar.set_margin(0, 0, 15, 15);
                    bar
                })
        ).unwrap();

        Ok(())
    }){}
}

fn read_port(tx: SyncSender<(String, String)>) {

    let port = serialport::new(PORT_NAME, BAUD_RATE)
        .timeout(Duration::from_millis(10))
        .open()
        .expect("Falha ao acessar porta.");

    let mut reader = BufReader::new(port);
    loop {
        let mut leitura = String::new();
        match reader.read_line(&mut leitura){
        Ok(_) => {
            // print!("{}", leitura);
            leitura.split("\r\n")
                .map(|s| s.split_ascii_whitespace().take(2).collect::<Vec<_>>())
                .filter(|s| s.len() == 2)
                .map(|a| (a[0].to_string(), a[1].to_string()))
                .filter(|(x, _)| x.is_ascii())
                .filter(|(x, _)| !x.starts_with("."))
                .filter(|(_, x)| !x.starts_with("."))
                .for_each(|(x, y)| {
                    match tx.send((x.clone(), y.clone())){
                        Ok(_) => {
                            // println!("{}:{}", x,y);
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            exit(1);
        }};});}
        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),//eprintln!("Reading Timeout!"),
        Err(e) => eprintln!("{:?}", e),
    }}}


fn interpolate_values(x1: f64, y1: f64, x2: f64, y2: f64, x: f64) -> f64 {
    y1 + (y2 - y1) * ((x - x1) / (x2 - x1))
}

fn interpolate_dictionary(dict: &Map<String, f64>) -> Map<i32, f64> {
    let mut interpolated_dict = Map::new();
    let keys: Vec<&String> = dict.keys().collect();

    for i in 0..keys.len() {
        let current_key = keys[i].parse::<f64>().unwrap();
        let current_value = dict[keys[i]];

        // Se a chave for um número inteiro, mantê-la como está
        if current_key.fract() == 0.0 {
            interpolated_dict.insert(current_key as i32, current_value);
        } else {
            // Encontrar as duas chaves mais próximas
            let mut j = i;
            while j > 0 && keys[j].parse::<f64>().unwrap() > current_key {
                j -= 1;
            }
            let key1 = keys[j - 1].parse::<f64>().unwrap();
            let value1 = dict[&keys[j - 1].to_string()];
            let key2 = keys[j].parse::<f64>().unwrap();
            let value2 = dict[&keys[j].to_string()];

            // Interpolar o valor correspondente
            let interpolated_value = interpolate_values(key1, value1, key2, value2, current_key);
            interpolated_dict.insert(current_key.round() as i32, interpolated_value);
        }
    }
    interpolated_dict
}