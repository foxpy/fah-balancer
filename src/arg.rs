// Copyright Murad Karammaev
// SPDX-License-Identifier: MIT

use crate::{
    error::{Error, Result},
    ncpu,
};
use std::{env, ops::RangeInclusive};

pub struct Arg {
    pub cpu_groups: Vec<CpuGroup>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CpuGroup {
    cpus: Vec<Cpu>,
    pub total_cpus: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Cpu {
    Single(usize),
    Range(RangeInclusive<usize>),
}

struct SeenCpus {
    seen_cpus: Vec<bool>,
}

impl SeenCpus {
    fn new() -> Result<Self> {
        Ok(Self {
            seen_cpus: vec![false; ncpu::ncpu()],
        })
    }

    fn mark(&mut self, cpu_id: usize) -> Result<()> {
        match self.seen_cpus.get_mut(cpu_id) {
            None => Err(Error::CpuIndexTooHigh),
            Some(seen) => {
                if *seen {
                    Err(Error::CpuIndexOverlaps)
                } else {
                    *seen = true;
                    Ok(())
                }
            }
        }
    }
}

impl Arg {
    pub fn parse() -> Result<Self> {
        let mut cpu_groups = env::args()
            .skip(1)
            .map(|arg| CpuGroup::try_from(arg.as_str()))
            .collect::<Result<Vec<_>>>()?;

        if cpu_groups.is_empty() {
            return Err(Error::NoCpuGroups);
        }

        // CPU groups with more total_cpus go last
        cpu_groups.sort_by_key(|a| a.total_cpus);

        let mut seen_cpus = SeenCpus::new()?;
        for group in &cpu_groups {
            for cpu in &group.cpus {
                match cpu {
                    Cpu::Single(id) => seen_cpus.mark(*id)?,
                    Cpu::Range(ids) => {
                        for id in ids.clone() {
                            seen_cpus.mark(id)?;
                        }
                    }
                }
            }
        }

        Ok(Self { cpu_groups })
    }
}

impl TryFrom<&str> for CpuGroup {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self> {
        let cpus = s
            .split(',')
            .map(Cpu::try_from)
            .collect::<Result<Vec<_>>>()?;
        let total_cpus = cpus
            .iter()
            .map(|cpu| match cpu {
                Cpu::Single(_) => 1,
                Cpu::Range(r) => r.end() - r.start() + 1,
            })
            .sum();
        Ok(Self { cpus, total_cpus })
    }
}

impl CpuGroup {
    pub fn cpus(&self) -> &[Cpu] {
        &self.cpus
    }
}

impl TryFrom<&str> for Cpu {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self> {
        if let Some((start, end)) = s.split_once('-') {
            let start = str::parse(start)?;
            let end = str::parse(end)?;
            if start >= end {
                return Err(Error::InvalidCpuRange);
            }

            Ok(Self::Range(start..=end))
        } else {
            Ok(Self::Single(str::parse(s)?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Cpu, CpuGroup};
    use crate::error::Error;
    use std::ops::RangeInclusive;

    fn s(id: usize) -> Cpu {
        Cpu::Single(id)
    }

    fn r(range: RangeInclusive<usize>) -> Cpu {
        Cpu::Range(range)
    }

    #[test]
    fn parse_cpu_success() {
        for (input, cpu) in [
            ("0", s(0)),
            ("1", s(1)),
            ("0-1", r(0..=1)),
            ("0-5", r(0..=5)),
            ("6-20", r(6..=20)),
        ] {
            assert_eq!(
                Cpu::try_from(input).unwrap(),
                cpu,
                "attempt to construct CPU from '{input}'"
            );
        }
    }

    #[test]
    fn parse_cpu_failure() {
        let f = |s: &str| Cpu::try_from(s).unwrap_err();

        assert!(matches!(f("-5"), Error::ParseInt(..)));
        assert!(matches!(f("0.1"), Error::ParseInt(..)));
        assert!(matches!(f("1-1"), Error::InvalidCpuRange));
        assert!(matches!(f("1-2-3"), Error::ParseInt(..)));
    }

    #[test]
    fn parse_cpu_group() {
        let tests = [
            ("0", &[s(0)][..], 1),
            ("0,1", &[s(0), s(1)][..], 2),
            ("0-1", &[r(0..=1)][..], 2),
            ("0-1,5,7,10-11", &[r(0..=1), s(5), s(7), r(10..=11)][..], 6),
        ];
        for (input, cpus, total_cpus) in tests {
            assert_eq!(
                CpuGroup::try_from(input).unwrap(),
                CpuGroup {
                    cpus: cpus.to_vec(),
                    total_cpus,
                },
                "attempt to construct CPU group from '{input}'"
            );
        }
    }
}
