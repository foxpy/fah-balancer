// Copyright Murad Karammaev
// SPDX-License-Identifier: MIT

use std::{error, fmt, io, num, result};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ParseInt(num::ParseIntError),
    Io(io::Error),
    InvalidCpuRange,
    NoCpuGroups,
    CpuIndexTooHigh,
    CpuIndexOverlaps,
    KernelIgnoredSchedSetAffinity,
    OutOfCpuGroups,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;
        match self {
            ParseInt(e) => write!(f, "{e}"),
            Io(e) => write!(f, "{e}"),
            InvalidCpuRange => write!(f, "Invalid CPU range"),
            NoCpuGroups => write!(f, "No CPU groups"),
            CpuIndexTooHigh => write!(f, "CPU index too high"),
            CpuIndexOverlaps => write!(f, "CPU index overlaps"),
            KernelIgnoredSchedSetAffinity => write!(f, "Kernel ignored sched_setaffinity()"),
            OutOfCpuGroups => write!(f, "Not enough CPU groups to fit FAH cores in"),
        }
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<num::ParseIntError> for Error {
    fn from(e: num::ParseIntError) -> Self {
        Self::ParseInt(e)
    }
}
