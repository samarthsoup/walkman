use rodio::{Decoder, OutputStream, Sink};
use std::{fs::File, io::{self, BufReader, Write}, thread, sync::mpsc};

enum InterruptMessage {
    Play(String),
    Queue(String),
    Stop,
    Pause,
    Resume,
    AudioFinished,
}

fn play_mp3(rx: mpsc::Receiver<InterruptMessage>, tx: mpsc::Sender<InterruptMessage>) -> Result<(), Box<dyn std::error::Error>> {
    let (_stream, stream_handle) = OutputStream::try_default()?;

    let mut sink: Option<Sink> = None;

    for msg in rx {
        match msg {
            InterruptMessage::Play(file_path) => {
                if let Some(ref s) = sink {
                    s.stop();
                }
                let file = match File::open(file_path){
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("io error: {}", e);
                        print!(": ");
                        io::stdout().flush().unwrap();
                        continue;
                    }
                };
                let source = match Decoder::new(BufReader::new(file)){
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("decoder error: {}", e);
                        print!(": ");
                        io::stdout().flush().unwrap();
                        continue; 
                    }
                };
                
                let s = match Sink::try_new(&stream_handle){
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("sink error: {}", e);
                        print!(": ");
                        io::stdout().flush().unwrap();
                        continue; 
                    }
                };
                s.append(source);
                sink = Some(s);
            },
            InterruptMessage::Queue(file_path) => {
                if sink.is_none() {
                    match Sink::try_new(&stream_handle) {
                        Ok(s) => sink = Some(s),
                        Err(e) => {
                            eprintln!("sink error(creation): {}", e);
                            print!(": ");
                            io::stdout().flush().unwrap();
                            continue;
                        }
                    };
                }
                let file = match File::open(file_path){
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("io error: {}", e);
                        print!(": ");
                        io::stdout().flush().unwrap();
                        continue;
                    }
                };
                match Decoder::new(BufReader::new(file)) {
                    Ok(source) => {
                        if let Some(ref s) = sink {
                            s.append(source);
                        }
                    },
                    Err(e) => {
                        eprintln!("decoder error: {}", e);
                        print!(": ");
                        io::stdout().flush().unwrap();
                        continue;
                    }
                };
            },
            InterruptMessage::Stop => {
                if let Some(ref s) = sink {
                    s.stop();
                }
                sink = None;
            },
            InterruptMessage::Pause => {
                if let Some(ref s) = sink {
                    s.pause();
                }
            },
            InterruptMessage::Resume => {
                if let Some(ref s) = sink {
                    s.play();
                }
            },
            InterruptMessage::AudioFinished => {},
        }
    }

    if let Some(ref s) = sink {
        if !s.empty() {
            thread::sleep(std::time::Duration::from_millis(100));
        } else {
            tx.send(InterruptMessage::AudioFinished).unwrap();
            sink = None;
        }
    }
    Ok(())
}

fn main() {
    let (audiotx, audiorx) = mpsc::channel();
    let (maintx, mainrx) = mpsc::channel();

    let maintx_clone = maintx.clone();

    thread::spawn(move || {
        match play_mp3(audiorx, maintx_clone){
            Ok(()) => {},
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    });

    loop{
        print!(": ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            eprintln!("read error");
            continue; 
        }
        let input = input.trim();

        match input {
            command if command.starts_with("p ") => {
                let file_path = format!(r"C:\Users\thesa\walkman\src\{}.mp3", command[2..].to_string());
                audiotx.send(InterruptMessage::Play(file_path)).unwrap();
            },
            command if command.starts_with("q ") => {
                let file_path = format!(r"C:\Users\thesa\walkman\src\{}.mp3", command[2..].to_string());
                audiotx.send(InterruptMessage::Queue(file_path)).unwrap();
            },
            "pz" => {
                audiotx.send(InterruptMessage::Pause).unwrap();
            },
            "r" => {
                audiotx.send(InterruptMessage::Resume).unwrap();
            },
            "s" => {
                audiotx.send(InterruptMessage::Stop).unwrap();
            },
            "h" => {
                println!("walkman docs\nplay: p {{songname}}\nqueue: q {{songname}}\npz: pause\nr: resume\nstop: s\nexit: e\ndocs: h");
            }
            "e" => {
                break;
            },
            _ => println!("invalid command"),
        }
    }    
}
