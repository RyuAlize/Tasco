use std::{
    fs::File,
    path::{PathBuf, Path}, 
    time::{Duration, Instant}, 
    io::BufReader
};

use flume::Sender;
use rodio::{Decoder, Devices, OutputStream, OutputStreamHandle, Sink, Source};
pub enum PlayStatus {
    Waiting,
    Playing(Instant, Duration),
    Paused(Instant, Instant, Duration),
    Complete,
}

pub struct PlayController {
    pub volume: f32,
    pub current_time: Duration,
    pub total_time: Duration,
    pub status: PlayStatus,
    pub playing_song: Option<String>,
    pub play_list: Vec<(String, PathBuf)>,
    pub playlist_index: usize,
    pub is_playing: bool,
    stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Sink,
}

impl PlayController {
    pub fn new() -> PlayController {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        Self {           
            volume: 1.0,
            current_time: Duration::from_secs(0),
            total_time: Duration::from_secs(0),
            status: PlayStatus::Waiting,
            playing_song: None,
            play_list: vec![],
            playlist_index: 0,
            is_playing: false,
            stream,
            stream_handle,
            sink,

        }
    }

    pub fn play_song(&mut self, song_path: &Path) -> bool {
        let duration: Duration;
        if song_path.extension().unwrap() == "mp3" {
            let dur = mp3_duration::from_path(song_path);
            match dur {
                Ok(dur) => {
                    duration = dur;
                }
                Err(err) => {
                    duration = err.at_duration;
                    if duration.is_zero() {
                        return false;
                    }
                }
            }
        } else {
            if let Ok(f) = File::open(song_path) {
                let dec = Decoder::new(f);
                if let Ok(dec) = dec {
                    if let Some(dur) = dec.total_duration() {
                        duration = dur;
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        match File::open(song_path) {
            Ok(f) => {
                let buf_reader = BufReader::new(f);
                self.sink = self.stream_handle.play_once(buf_reader).unwrap();
                self.sink.set_volume(self.volume);
                self.total_time = duration;
                self.current_time = Duration::from_secs(0);
                self.status = PlayStatus::Waiting;
                self.is_playing = false;
                self.play();
                return true;
                
            }
            Err(_) => false
        }
    }

    pub fn inc_vol(&mut self) {
        if self.volume < 1.0{
            self.volume += 0.01;
            self.sink.set_volume(self.volume);
            
        }
    }

    pub fn dec_vol(&mut self) {
        if self.volume > 0.0 {
            self.volume -= 0.01;
            self.sink.set_volume(self.volume);
        }
    }

    pub fn is_playing_or_paused(&self) -> bool {
        match &self.status {
            PlayStatus::Waiting | PlayStatus::Complete => false,
            _ => true
        }
    }

    pub fn play(&mut self) {
        self.sink.play();
        self.is_playing = true;
        let ref mut status = self.status;
        match status {
            PlayStatus::Complete | PlayStatus::Waiting => {
                *status = PlayStatus::Playing(Instant::now(), Duration::from_secs(0));
            },
            PlayStatus::Paused(start_ins, pause_ins, paused_time) => {
                *paused_time += pause_ins.elapsed();
                *status = PlayStatus::Playing(start_ins.to_owned(), paused_time.to_owned());
            }
            _ =>()
        }
    }

    pub fn pause(&mut self) {
        self.sink.pause();
        self.is_playing = false;
        let ref mut status = self.status;
        match status {
            PlayStatus::Playing(start_ins, paused_time) => {
                self.status = PlayStatus::Paused(start_ins.to_owned(), Instant::now(), paused_time.to_owned());
            }
            _ => ()
        }
    }

    pub fn next(&mut self) {
        if self.play_list.len() == 0 {return;}
        if self.playlist_index == self.play_list.len()-1 {
            self.playlist_index = 0;
        }
        else{
            self.playlist_index += 1;
        }
        let (mut name, mut path)  = self.play_list[self.playlist_index].clone();
        self.playing_song = Some(name); 
        self.play_song(path.as_path());
    }

    pub fn tick(&mut self) {
        let ref mut status = self.status;
        match status {
            PlayStatus::Playing(start_ins, paused_time) => {
                let now = start_ins.elapsed() - *paused_time;
                if now.ge(&self.total_time) {
                    *status = PlayStatus::Complete;               
                    self.next();
                }
                else{
                    self.current_time = now;
                }
            },
            _ =>()
        }
    }

    pub fn get_progress(&self) -> String{
        let current_time = self.current_time;
        let total_time = self.total_time;

        let minute_mins = current_time.as_secs() / 60;
        let minute_secs = current_time.as_secs() % 60;

        let total_mins = total_time.as_secs() / 60;
        let total_secs = total_time.as_secs() % 60;
        format!("{:0>2}:{:0>2} / {:0>2}:{:0>2}",
            minute_mins, minute_secs, total_mins, total_secs).to_owned()
    }
}
