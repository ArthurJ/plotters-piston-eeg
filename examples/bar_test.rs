use piston_window::{EventLoop, PistonWindow, WindowSettings};
use plotters_piston_eeg::draw_piston_window;
use std::collections::BTreeMap as Map;
use plotters::prelude::*;
use std::io::{self, BufRead, BufReader};
use std::str::FromStr;
use std::time::Duration;
use serialport::SerialPort;

const FPS: u32 = 30;

fn main() {

    let mut window: PistonWindow = WindowSettings::new("Frequências em Tempo Real", [900, 600])
        .samples(4)
        .build()
        .unwrap();

    window.set_max_fps(FPS as u64);

    // let port_name = "/dev/ttyUSB0";
    let port_name = "/dev/ttyACM0";
    let baud_rate = 115200;
    let freq_quantity = 36;

    let mut port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(10))
        .open()
        .expect("Falha ao acessar porta.");

    while let Some(_) = draw_piston_window(&mut window, |b| {
        let values = read_port(&mut port);
        println!("Final values: {:?}\nlen:{}", values, values.len());
        // println!("Final frequencies: {:?}\nlen:{}",
        //          values.keys().into_iter().map(|x| f32::from_str(x).unwrap_or(-1.0)).collect::<Vec<_>>(),
        //          values.len());

        let root = b.into_drawing_area();
        root.fill(&WHITE)?;

        let mut ctx = ChartBuilder::on(&root)
            .margin(40)
            .caption("FFT", ("sans-serif", 40))
            .set_label_area_size(LabelAreaPosition::Left, 40)
            .set_label_area_size(LabelAreaPosition::Bottom, 40)
            .set_label_area_size(LabelAreaPosition::Right, 40)
            .build_cartesian_2d((0..freq_quantity).into_segmented(), 0..3000)
            .unwrap();

        ctx.configure_mesh()
            .x_desc("Frequências")
            .y_desc("Magnitude")
            .axis_desc_style(("sans-serif", 20))
            .draw().unwrap();

        ctx.draw_series(values.iter()
            .map(|(x, y)| (f32::from_str(x).unwrap_or(-1.0), f32::from_str(y).unwrap_or(-1.0)))
            .map(|(x, y)| {
                let x0 = SegmentValue::Exact(x.round() as i32);
                let x1 = SegmentValue::Exact(x.round() as i32 + 1);
                let mut bar = Rectangle::new([(x0, 0), (x1, y as i32)], RED.filled());
                bar.set_margin(0, 0, 15, 15);
                bar
            }))
            .unwrap();

        Ok(())
    }){}
}

fn read_port(port: &mut Box<dyn SerialPort>) -> Map<String, String> {
    let mut reader = BufReader::new(port);
    let mut leitura = String::new();
    let mut value_map = Map::new();
    loop {
        match reader.read_line(&mut leitura){
            Ok(_) => {
                leitura.split("\r\n")
                    .map(|s| s.split_ascii_whitespace().take(2).collect::<Vec<_>>())
                    .filter(|s| s.len() == 2)
                    .map(|a| (a[0].to_string(), a[1].to_string()))
                    .filter(|(x, _)| x.is_ascii())
                    .filter(|(x, _)| !x.starts_with("."))
                    .filter(|(_, x)| !x.starts_with("."))
                    .for_each(|(x, y)| { value_map.insert(x, y); });

                // println!("Final values: {:?}\nlen:{}", value_map, value_map.len());

                if leitura.contains("\r\n\r\n") { //Controle do tamanho do buffer
                    return value_map;
                }}
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }}}