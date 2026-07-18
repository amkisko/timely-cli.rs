//! Process exit codes for script-friendly error handling.

use std::process::ExitCode;

use timely_lib::TimelyError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AppExit {
    Success = 0,
    General = 1,
    Usage = 2,
    Auth = 3,
    Api = 4,
    Io = 5,
}

impl AppExit {
    pub fn code(self) -> ExitCode {
        ExitCode::from(self as u8)
    }
}

pub fn exit_for_timely_error(error: &TimelyError) -> AppExit {
    match error {
        TimelyError::Auth(_) => AppExit::Auth,
        TimelyError::Api(_) => AppExit::Api,
        TimelyError::Usage(_) => AppExit::Usage,
        TimelyError::Io(_) => AppExit::Io,
        TimelyError::Other(_) => AppExit::General,
    }
}

pub fn exit_for_message(message: &str) -> AppExit {
    timely_lib::error::classify_message(message).into()
}

impl From<TimelyError> for AppExit {
    fn from(error: TimelyError) -> Self {
        exit_for_timely_error(&error)
    }
}
