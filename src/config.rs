
pub const Version: &str = "1.0";
pub const Music_file_extensions: [&str;4] = ["mp3", "wav", "flac", "ts"];
pub struct Config {
    pub fresh_time: u64,

}

impl Config {
    pub fn default() -> Self {
        Self{
            fresh_time:100,
        }
    }
}


#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum RawKey {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Backspace,
    Enter,
    Tab,
    Home,
    End,
    PageUp,
    PageDown,
    BackTab,
    Delete,
    Insert,
    Null,
    Esc,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum KeyBinding {
    Ctrl(RawKey),
    Shift(RawKey),
    Raw(RawKey),
    F(u8),
    Unsupported,
}
