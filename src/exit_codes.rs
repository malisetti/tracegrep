#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    Match = 0,
    NoMatch = 1,
    Error = 2,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

impl From<bool> for ExitCode {
    fn from(any_match: bool) -> Self {
        if any_match {
            ExitCode::Match
        } else {
            ExitCode::NoMatch
        }
    }
}
