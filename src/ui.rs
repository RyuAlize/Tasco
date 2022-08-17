use std::io::{Error};
use std::path::{Path, PathBuf};
use rand::Rng;
use tui::{
    backend::{CrosstermBackend, Backend},
    layout::{Alignment, Rect, Layout, Constraint, Direction}, 
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap, ListState, LineGauge, BarChart},
    text::{Text, Span, Spans},
    style::{Color, Style, Modifier,},
    symbols,
    Frame,
    Terminal
};

use crate::play_controller::PlayController;
use crate::config::Config;
pub struct UI {
    curr_dir: CurrDir,
    control_bar: ControlBar,
    curr_song: CurrSong, 
    explore: Explorer,
    playlist: PlayList,
    effect_bar: EffectivenessBar,
    process_bar: ProcessBar,
}

impl UI 
{
    pub fn new<B: Backend>(config: &Config, terminal: &Terminal<B>) -> Result<UI, Error> {
        let terminal_size = terminal.size()?;
        let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Percentage(100)].as_ref())
                .split(terminal_size);
        let header_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(60),Constraint::Percentage(20)].as_ref())
                .split(layout[0]);
        let body_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(60),Constraint::Percentage(20)].as_ref())
            .split(layout[1]);
        let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(90), Constraint::Length(10)].as_ref())
        .split(body_layout[1]);
        
        Ok(Self{
            curr_dir: CurrDir::new(header_layout[0]),
            control_bar: ControlBar::new(header_layout[1]),
            curr_song: CurrSong::new(header_layout[2]),
            explore: Explorer::new(body_layout[0]),
            playlist: PlayList::new(body_layout[2]),
            effect_bar: EffectivenessBar::new(chunks[0]),
            process_bar: ProcessBar::new(chunks[1]),
        })
    }


    pub fn draw_ui<B: Backend>(&self, 
        terminal: &mut Terminal<B>, 
        explorer_list: &Vec<(String, PathBuf)>,
        play_list: &Vec<(String, PathBuf)>,
        dir_name: Option<&str>,
        playing_song: Option<&String>,
        explore_index: usize,
        playlist_index: usize,
        player: &PlayController,
        ) -> Result<(), Error> 
    {
        terminal.draw(|frame| {
            self.draw_explorer(frame, explorer_list, dir_name, explore_index);
            self.control_bar.draw(frame);
            self.draw_playlist(frame, play_list, playing_song, playlist_index);
            self.effect_bar.draw(frame, player);
            self.process_bar.draw(frame, player);
        })?;
        Ok(())
    }

    pub fn draw_explorer<B: Backend>(&self, 
        frame: &mut Frame<B>,
        explorer_list: &Vec<(String, PathBuf)>, 
        dir_name: Option<&str>,
        index: usize) 
    {
        self.curr_dir.draw(frame, dir_name);
        self.explore.draw(frame, explorer_list, index);
    }

    pub fn draw_playlist<B: Backend>(&self, 
        frame: &mut Frame<B>, 
        play_list: &Vec<(String, PathBuf)>, 
        playing_song: Option<&String>, 
        index: usize) 
    {
        self.curr_song.draw(frame, playing_song);
        self.playlist.draw(frame, play_list, index);
    }

}


struct PlayList {
    area: Rect,
    index: usize,
} 

impl PlayList {
    pub fn new(area: Rect) -> PlayList {
        Self { area, index: 0 }
    }

    pub fn draw<B: Backend>(&self, frame: &mut Frame<B>, play_list: &Vec<(String, PathBuf)>, index: usize) {
        let mut items = vec![];
        let mut list_state = ListState::default();
        for item in play_list {
            items.push(ListItem::new(item.0.as_str()));
        }
        if items.len() > 0 { list_state.select(Some(index));}
        let block = Block::default()
            .title("Playlist")
            .borders(Borders::ALL)
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);         
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::Cyan))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, self.area.clone(), &mut list_state);
    }
}

struct Explorer {
    area: Rect,
}

impl Explorer {
    pub fn new(area: Rect) -> Explorer {
        Self { 
            area,
        }
    }

    pub fn draw<B: Backend>(&self, frame: &mut Frame<B>, explorer_list: &Vec<(String, PathBuf)>, index: usize) {
        let mut items = vec![ListItem::new("Go Back")];
        for item in explorer_list {
            items.push(ListItem::new(item.0.as_str()));
        }
        let block = Block::default()
            .title("Explorer")
            .borders(Borders::ALL)
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);         
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::Cyan))
            .highlight_symbol("> ");
        
        let mut list_state = ListState::default();
        list_state.select(Some(index));
        frame.render_stateful_widget(list, self.area.clone(), &mut list_state);
    }


}
struct ProcessBar {
    progress_area: Rect,
    vol_area: Rect,
}

impl ProcessBar {
    pub fn new(area: Rect) -> ProcessBar {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
            .split(area);
        Self { 
            progress_area: layout[0],
            vol_area: layout[1]
        }
    }

    pub fn draw<B: Backend>(&self, frame: &mut Frame<B>, player: &PlayController) {
        self.draw_vol(frame, player.volume as f64);
        
        if player.is_playing_or_paused() {
            let precent = player.current_time.as_secs_f64() / player.total_time.as_secs_f64();
            let progress = player.get_progress();
            self.draw_progress(frame, Some(progress), precent);
        }
        else{
            self.draw_progress(frame, None, 0.0);
        }
        
    }
    
    pub fn draw_progress <B: Backend>(&self, frame: &mut Frame<B>, progress: Option<String>, percent: f64) {
        let mut s = "No More Sound".to_string();
        if let Some(progress) = progress {
            s = progress;
        }  
        let gauge = LineGauge::default()
            .ratio(percent)
            .line_set(symbols::line::THICK)
            .label(s)
            .style(Style::default().add_modifier(Modifier::ITALIC))
            .gauge_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
        let layout = Layout::default()
            .margin(1)
            .horizontal_margin(1)
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(self.progress_area.clone());
        frame.render_widget(gauge, layout[0]);
    } 

    pub fn draw_vol<B: Backend>(&self, frame: &mut Frame<B>, vol: f64) {
        let bar = LineGauge::default()
            .ratio(vol)
            .label("VOL")
            .line_set(symbols::line::THICK)
            .block(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::LEFT | Borders::RIGHT),
            )
            .gauge_style(
                Style::default()
                    .fg(Color::LightCyan)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );
            let layout = Layout::default()
            .margin(1)
            .horizontal_margin(1)
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(self.vol_area.clone());
        frame.render_widget(bar, layout[0]);
    }
}

struct EffectivenessBar {
    area: Rect,
}

impl EffectivenessBar {
    pub fn new(area: Rect) -> EffectivenessBar {
        Self { area }
    }

    pub fn draw<B>(&self, frame: &mut Frame<B>, player: &PlayController)
    where
        B: Backend
    {
        let mut rng = rand::thread_rng();
        let mut cols = vec![];
        for _ in 0..20 {
            let mut i = rng.gen_range(0..10);
            if !player.is_playing {
                i = 0
            }
            cols.push(("_", i));
        }
        let items = BarChart::default()
                .bar_width(4)
                .bar_gap(1)
                .bar_style(Style::default().fg(Color::Cyan).bg(Color::Black))
                .data(&cols)
                .value_style(Style::default().add_modifier(Modifier::ITALIC))
                .label_style(Style::default().add_modifier(Modifier::ITALIC))
                .max(10)
                .block(
                    Block::default()
                        .borders(Borders::TOP | Borders::BOTTOM)
                        .border_type(BorderType::Double)
                        .title("Wave")
                        .title_alignment(Alignment::Center),
                );  
        frame.render_widget(items, self.area.clone());
    }
}

struct ControlBar {
    area: Rect,
}

impl ControlBar {
    pub fn new(area: Rect) -> ControlBar {
        Self { area }
    }

    pub fn draw<B>(&self, frame: &mut Frame<B>)
    where
        B: Backend
    {
        let mut p = Paragraph::new(vec![Spans::from("▶(s) >>|(n) EXT(q) HLP(h)")])
            .style(Style::default())
            .alignment(Alignment::Center);
        let block = Block::default()
            .title("Control")
            .borders(Borders::ALL)
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(Color::LightBlue));
        p = p.block(block);
        
        frame.render_widget(p, self.area.clone());
    }
}

struct CurrDir {
    area: Rect,
}

impl CurrDir {
    pub fn new(area: Rect) -> CurrDir {
        Self { area }
    }

    pub fn draw<B: Backend>(&self, frame: &mut Frame<B>, current_dir: Option<&str>) {
        let mut dir_text = "".to_string();
        if let Some(text) = current_dir {
            dir_text = text.to_owned();
        }
        else{
            dir_text = "None".to_string();
        }
        let text = Paragraph::new(dir_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Directory")
                .title_alignment(Alignment::Center),
        )
        .alignment(Alignment::Center)
        .style(Style::default().add_modifier(Modifier::BOLD));
    
        frame.render_widget(text, self.area.clone());
    }
}

struct CurrSong {
    area: Rect,
}

impl CurrSong {
    pub fn new(area: Rect) -> CurrSong {
        Self { area }
    }

    pub fn draw<B: Backend>(&self, frame: &mut Frame<B>, playing_song: Option<&String>) {
        let mut playing_text = "".to_string();
        if let Some(text) = playing_song {
            playing_text = text.clone();
        }
        else{
            playing_text = "None".to_string();
        }
        let text = Paragraph::new(playing_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Now Playing")
                .title_alignment(Alignment::Center),
        )
        .alignment(Alignment::Center)
        .style(Style::default().add_modifier(Modifier::BOLD));
    
        frame.render_widget(text, self.area.clone());
    }
}