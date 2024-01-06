use rodio::{Decoder, OutputStream, Sink, StreamError, PlayError, decoder::DecoderError};
use std::{fs::File, io::{self, BufReader}, fmt, error::Error, thread, sync::mpsc};


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
    Stop,
    AudioFinished,
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
            InterruptMessage::Stop => {
                if let Some(ref s) = sink {
                    s.stop();
                }
                sink = None;
            },
            InterruptMessage::AudioFinished => {}
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
            Ok(()) => println!("playing track"),
            Err(e) => eprintln!("{}", e),
        }
    });

    loop{
        println!("enter the input of the song to play (or 'stop' to stop(or 'exit' to kill the program)):");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("error 1: couldn't read line");
        let input = input.trim();

        match input {
            command if command.starts_with("play ") => {
                let file_path = format!(r"C:\Users\thesa\walkman\src\{}.mp3", command[5..].to_string());
                audiotx.send(InterruptMessage::Play(file_path)).unwrap();
            },
            "stop" => {
                audiotx.send(InterruptMessage::Stop).unwrap();
            },
            "exit" => {
                break;
            },
            _ => println!("invalid command"),
        }
        if let Ok(notification) = mainrx.try_recv() {
            match notification {
                InterruptMessage::AudioFinished => {
                    println!("Audio finished playing.");
                },
                InterruptMessage::Play(_) => {},
                InterruptMessage::Stop => {},
            }
        }
    }    
}
