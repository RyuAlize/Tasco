mod config;
mod ui;
mod play_controller;
mod music_player;

use clap::{App, Arg};
use std::{io, thread, time::Duration};
use config::Config;

use ui::UI;
use music_player::MusicPlayer;
use config::Version;

fn main() -> Result<(), io::Error> {

    let cli = App::new("MusicPlayer")
        .version(Version)
        .author("Author: RyuAlize <https://github.com/RyuAlize>")
        .about("A terminal music player written in Rust.")
        .arg(
            Arg::with_name("dir")
            .multiple(true)           
                .takes_value(true)
                .help(
                    r#"The directory of music files"#,
                )
        );


    match MusicPlayer::new(cli, Config::default()) {
        Ok(mut app) => {
            app.run()?;
            app.destruct()?;
        },
        Err(_) => {println!("Invalid directory path!")}
    }
    
    Ok(())
}

