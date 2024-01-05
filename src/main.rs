use rodio::{Decoder, OutputStream, Sink, StreamError, PlayError, decoder::DecoderError};
use std::{fs::File, io::{self, BufReader}, fmt, error::Error, thread, sync::mpsc, time::Duration};


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
    AudioFinished,
    Exit,
}

fn play_mp3(file_path: &str, tx: mpsc::Sender<InterruptMessage>) -> Result<(), ErrorKind> {
    let (_stream, stream_handle) = OutputStream::try_default()?;

    let file = File::open(file_path)?;
    let source = Decoder::new(BufReader::new(file))?;

    let sink = Sink::try_new(&stream_handle)?;
    sink.append(source);
    sink.sleep_until_end();

    tx.send(InterruptMessage::AudioFinished).unwrap();
    Ok(())
}

fn main() {
    let (tx, rx) = mpsc::channel();

    loop {
        println!("enter the input of the song to play (or 'exit' to stop):");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("error 1: couldn't read line");
        let input = input.trim();

        match input {
            "exit" => {
                tx.send(InterruptMessage::Exit).unwrap();
                break;
            },
            _ => {
                let file_path = format!(r"C:\Users\thesa\walkman\src\{}.mp3", input);
                let tx_clone = tx.clone();
                thread::spawn(move || {
                    match play_mp3(&file_path, tx_clone){
                        Ok(()) => println!("playing track"),
                        Err(e) => eprintln!("{}", e),
                    }
                });
            }
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(InterruptMessage::AudioFinished) => println!("exhausted the byte stream"),
            Ok(InterruptMessage::Exit) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
        }
    }
}
