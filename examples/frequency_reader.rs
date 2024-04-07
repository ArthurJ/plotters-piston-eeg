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

    let port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(10))
        .open();

    let mut port = match port {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Falha ao acessar porta: {:?}", e);
            exit(1);
        }
    };

    let mut reader = BufReader::new(port);
    loop{
        let mut leitura = String::new();
        match reader.read_line(&mut leitura) {
            Ok(_) => {
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
            Err(ref e)
                if e.kind() == io::ErrorKind::BrokenPipe => {
                    eprintln!("{:?}", e);
                    exit(1);
            },
            Err(e) => eprintln!("{:?}", e)
        }
    }
}

fn main() {}