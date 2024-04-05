use std::io;
use std::io::{BufRead, BufReader};
use std::process::exit;
use std::str::FromStr;
use std::sync::mpsc::{SyncSender};
use std::time::Duration;
use serialport::SerialPort;


pub fn read_port(sender: SyncSender<isize>) {

    let port_name = "/dev/ttyUSB0";
    // let port_name = "/dev/ttyACM0";
    let baud_rate = 115200;

    let mut port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(10))
        .open()
        .expect("Falha ao acessar porta.");

    let mut reader = BufReader::new(port);
    loop{
        let mut leitura = String::new();
        match reader.read_line(&mut leitura) {
            Ok(_) => {
                // print!("{}", leitura);
                leitura.split("\n")
                    .filter(|x| !x.trim().is_empty())
                    .map(|x| isize::from_str(x.trim()).unwrap())
                    .for_each(|x|
                        {
                            match sender.send(x){
                                Ok(_) => {}
                                Err(e) => {
                                    eprintln!("Error: {}", e);
                                    exit(1);
                                }
                            };
                        });
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            // Err(ref e) if e.kind() == io::ErrorKind::TimedOut => eprintln!("Reading Timeout!"),
            Err(e) => eprintln!("{:?}", e)
            // println!("{:?}", valores);
        }
    }
}

fn main() {}