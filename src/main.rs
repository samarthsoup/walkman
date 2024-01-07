use rodio::{Decoder, OutputStream, Sink, StreamError, PlayError, decoder::DecoderError};
use std::{fs::File, io::{self, BufReader, Write}, fmt, error::Error, thread, sync::mpsc};


#[derive(Debug)]
enum ErrorKind {
    Io(io::Error),
    Stream(StreamError),
    Decoder(DecoderError),
    Play(PlayError)
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Io(err) => write!(f, "io error: {}", err),
            ErrorKind::Stream(err) => write!(f, "stream error: {}", err),
            ErrorKind::Decoder(err) => write!(f, "decoder error: {}", err),
            ErrorKind::Play(err) => write!(f, "play error: {}", err),
        }
    }
}

impl Error for ErrorKind {}

impl From<io::Error> for ErrorKind {
    fn from(err: io::Error) -> ErrorKind {
        ErrorKind::Io(err)
    }
}

impl From<StreamError> for ErrorKind {
    fn from(err: StreamError) -> ErrorKind {
        ErrorKind::Stream(err)
    }
}

impl From<DecoderError> for ErrorKind {
    fn from(err: DecoderError) -> Self {
        ErrorKind::Decoder(err)
    }
}

impl From<PlayError> for ErrorKind {
    fn from(err: PlayError) -> Self {
        ErrorKind::Play(err)
    }
}

enum InterruptMessage {
    Play(String),
    Queue(String),
    Stop,
    Pause,
    Resume,
    AudioFinished,
    UserError(String),
}

fn play_mp3(rx: mpsc::Receiver<InterruptMessage>, tx: mpsc::Sender<InterruptMessage>) -> Result<(), ErrorKind> {
    let (_stream, stream_handle) = OutputStream::try_default()?;

    let mut sink: Option<Sink> = None;

    for msg in rx {
        match msg {
            InterruptMessage::Play(file_path) => {
                if let Some(ref s) = sink {
                    s.stop();
                }
                let file = File::open(file_path)?;
                let source = Decoder::new(BufReader::new(file))?;
                
                let s = Sink::try_new(&stream_handle)?;
                s.append(source);
                sink = Some(s);
            },
            InterruptMessage::Queue(file_path) => {
                if sink.is_none() {
                    sink = Some(Sink::try_new(&stream_handle)?); 
                }
                let file = File::open(file_path)?;
                let source = Decoder::new(BufReader::new(file))?;
                sink.as_ref().unwrap().append(source);
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
            InterruptMessage::UserError(_) => {},
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
    let (mut audiotx, mut audiorx) = mpsc::channel();
    let (maintx, mainrx) = mpsc::channel();

    let mut maintx_clone = maintx.clone();
    let maintx_clone1 = maintx.clone();

    thread::spawn(move || {
        match play_mp3(audiorx, maintx_clone){
            Ok(()) => {},
            Err(e) => {
                eprintln!("{}", e);
                maintx_clone1.send(InterruptMessage::UserError(e.to_string())).unwrap();
            }
        }
    });

    loop{
        if let Ok(message) = mainrx.try_recv() {
            match message {
                InterruptMessage::UserError(_) => {},
                InterruptMessage::AudioFinished => {},
                InterruptMessage::Queue(_) => {},
                InterruptMessage::Play(_) => {},
                InterruptMessage::Stop => {},
                InterruptMessage::Pause => {},
                InterruptMessage::Resume => {},
            }
        }

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
                if audiotx.send(InterruptMessage::Play(file_path)).is_err(){
                    let new_channel = mpsc::channel();
                    audiotx = new_channel.0;
                    audiorx = new_channel.1;
                    maintx_clone = maintx.clone();

                    thread::spawn(move || {
                        println!("new thread created");
                        match play_mp3(audiorx, maintx_clone){
                            Ok(()) => {},
                            Err(e) => eprintln!("{}", e),
                        }
                    });
                };
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
