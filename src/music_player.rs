use std::io::{self, Error, ErrorKind, Stdout};
use std::fs::{self, DirEntry, ReadDir};
use std::thread;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::ffi::{OsStr, OsString};
use std::time::Duration;
use clap::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as InputEvent, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, Terminal};
use flume::{Sender, Receiver};

use crate::config::{Music_file_extensions, Config, RawKey, KeyBinding};
use crate::ui::UI;
use crate::play_controler::{PlayControler, PlayStatus};

pub struct MusicPlayer {
    config: Config,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    term_ui: UI,
    player: PlayControler,
    current_dir: PathBuf,
    explorer_list: Vec<(String, PathBuf)>,   
    explorer_index: usize,
    quit: bool,
}

impl MusicPlayer {
    pub fn new(args:App,config: Config) -> Result<MusicPlayer, Error> {
        let args = args.get_matches();
        let music_dir = args.value_of("dir").unwrap_or_default();
        let current_dir = PathBuf::from(music_dir);
        if !current_dir.is_dir() || !current_dir.exists() {
            return Err(Error::from(io::ErrorKind::InvalidInput));
        }
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;
        let  term_ui = UI::new(&config, &terminal)?;
        Ok(Self { 
            config, 
            terminal,
            term_ui,
            current_dir,
            explorer_list: vec![],
            explorer_index: 0,
            quit: false,
            player: PlayControler::new(),
        })
    }

    pub fn run(&mut self) -> Result<(), Error> {
        let (path_list, name_list) =  self.read_dir_and_music(self.current_dir.as_path())?;
        self.explorer_list = name_list.into_iter().zip(path_list.into_iter()).collect();
        self.draw_ui();      
        while !self.quit {
            self.process_input()?;
            self.player.tick();
            self.draw_ui()?;
        }
        Ok(())      
    }

    fn draw_ui(&mut self) -> Result<(), Error> {
        self.term_ui.draw_ui(&mut self.terminal, 
            &self.explorer_list, 
            &self.player.play_list,
            self.current_dir.to_str(),
            self.player.playing_song.as_ref(),
            self.explorer_index,
            self.player.playlist_index,
            &self.player
        )
    }

    fn read_dir_and_music(&self, dir_path: &Path) -> Result<(Vec<PathBuf>, Vec<String>), Error>{
        let mut path_list = vec![];
        let mut name_list = vec![];
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let tp =  entry.file_type()?;
            if let Ok(name) = entry.file_name().into_string() {
                if tp.is_dir() {
                    path_list.push(entry.path());
                    name_list.push(name);
                }
                else if tp.is_file() {
                    if let Some(file_extension) = entry.path().extension().and_then(OsStr::to_str){
                        if Music_file_extensions.contains(&file_extension) {
                            path_list.push(entry.path());
                            name_list.push(name);
                        }
                    }
                }
            }
            
        }
        Ok((path_list, name_list))
    }

    fn process_input(&mut self) -> Result<(), Error>{
        match self.read_event() {
            Some(InputEvent::Key(key)) => self.process_key(key)?,
            _ =>()
        }
        Ok(())
    }

    fn read_event(&mut self) -> Option<InputEvent> {
    
        if let Ok(true) = crossterm::event::poll(Duration::from_millis(self.config.fresh_time)) {
            if let Ok(key) = crossterm::event::read() {
                return Some(key);
            }
        } 
        None
    }

    fn process_key(&mut self, key: KeyEvent) -> Result<(), Error>{
        let key_bind = Self::key_event_to_player_key(key.code, key.modifiers);
        match key_bind {
            KeyBinding::Raw(RawKey::Up) => {
                self.explorer_up(); 
            },
            KeyBinding::Raw(RawKey::Down) => {
                self.explorer_down();       
            },
            KeyBinding::Raw(RawKey::Left) => {
                self.player.dec_vol();
            },
            KeyBinding::Raw(RawKey::Right) => {
                self.player.inc_vol()
            },
            KeyBinding::Raw(RawKey::Char(' ')) => {
                if self.player.is_playing_or_paused() {
                    if self.player.is_playing {
                        self.player.pause();
                    }
                    else{
                        self.player.play();
                    }
                }
            },
            KeyBinding::Raw(RawKey::Char('s')) => {
                if self.player.play_list.len() > 0 {
                    let selected_song = &self.player.play_list[self.player.playlist_index];
                    self.player.playing_song = Some(selected_song.0.clone()); 
                    self.change_song(selected_song.1.clone())?;
                }
            },
            KeyBinding::Raw(RawKey::Char('c')) => {
                self.player.playing_song = None;
                self.player.play_list.clear();
            },
            KeyBinding::Raw(RawKey::Char('n')) => {
                self.player.next();
            },
            KeyBinding::Raw(RawKey::Char('q')) => {
                self.quit = true;
            },
            KeyBinding::Raw(RawKey::Enter) => {
                if self.explorer_index == 0 {            
                    if self.current_dir.pop() {
                        self.change_directory(self.current_dir.clone())?;
                    }
                }
                else {
                    let selected = self.explorer_list[self.explorer_index-1].clone();
                    if selected.1.is_dir() {
                        self.change_directory(selected.1)?;
                    }
                    else{
                        self.append_to_playlist(selected)?;
                    }               
                }
            },
            KeyBinding::Shift(RawKey::Up) => {
                self.playerlist_up();

            },
            KeyBinding::Shift(RawKey::Down) => {
                self.playerlist_down();
            },
            _ => ()
        }
        Ok(())
    }
    
    fn key_event_to_player_key(key: KeyCode, modifiers: KeyModifiers) -> KeyBinding {
        // Convert crossterm's complicated key structure into simpler one
        let inner = match key {
            KeyCode::Char(c) => RawKey::Char(c),
            KeyCode::BackTab => RawKey::BackTab,
            KeyCode::Insert => RawKey::Insert,
            KeyCode::Esc => RawKey::Esc,
            KeyCode::Backspace => RawKey::Backspace,
            KeyCode::Tab => RawKey::Tab,
            KeyCode::Enter => RawKey::Enter,
            KeyCode::Delete => RawKey::Delete,
            KeyCode::Null => RawKey::Null,
            KeyCode::PageUp => RawKey::PageUp,
            KeyCode::PageDown => RawKey::PageDown,
            KeyCode::Home => RawKey::Home,
            KeyCode::End => RawKey::End,
            KeyCode::Up => RawKey::Up,
            KeyCode::Down => RawKey::Down,
            KeyCode::Left => RawKey::Left,
            KeyCode::Right => RawKey::Right,
            KeyCode::F(i) => return KeyBinding::F(i),
        };
        match modifiers {
            KeyModifiers::CONTROL => KeyBinding::Ctrl(inner),
            KeyModifiers::SHIFT => KeyBinding::Shift(inner),
            KeyModifiers::NONE => KeyBinding::Raw(inner),
            _ => KeyBinding::Unsupported,
        }
    }

    fn explorer_up(&mut self) {
        if self.explorer_index == 0 {
            self.explorer_index = self.explorer_list.len()
        }
        else{
            self.explorer_index -=1;
        }
    }

    fn explorer_down(&mut self) {
        if self.explorer_index == self.explorer_list.len() {
            self.explorer_index = 0;
        }
        else{
            self.explorer_index +=1;
        }
    }

    fn playerlist_up(&mut self) {
        if self.player.playlist_index == 0 {
            self.player.playlist_index = self.player.play_list.len()-1;
        }
        else {
            self.player.playlist_index -=1;
        }
    }

    fn playerlist_down(&mut self) {
        if self.player.playlist_index == self.player.play_list.len()-1 {
            self.player.playlist_index = 0;
        }
        else {
            self.player.playlist_index +=1;
        }
    }

    fn append_to_playlist(&mut self, selected_song: (String, PathBuf)) -> Result<(), Error> {
        self.player.play_list.push(selected_song);
        Ok(())
    }

    fn change_directory(&mut self, dir_path: PathBuf) -> Result<(), Error> {
        self.current_dir = dir_path;
        let (path_list, name_list) =  self.read_dir_and_music(self.current_dir.as_path())?;
        self.explorer_list = name_list.into_iter().zip(path_list.into_iter()).collect();
        self.explorer_index = 0;
        Ok(())
    }

    fn change_song(&mut self, song_path: PathBuf) -> Result<(), Error> {
        match self.player.play_song(song_path.as_path()){
            true =>(),
            false => panic!("failed to play")
        }
        Ok(())
    }
    pub fn destruct(mut self) -> Result<(), Error>{
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
    
}
