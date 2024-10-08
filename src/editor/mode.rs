use std::fmt::Display;

#[derive(Clone, PartialEq, Debug)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Normal => f.write_str("NOR"),
            Mode::Insert => f.write_str("INS"),
            Mode::Visual => f.write_str("VIS"),
        }
    }
}
