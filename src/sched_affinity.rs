// Copyright Murad Karammaev
// SPDX-License-Identifier: MIT

use crate::{
    arg::{Cpu, CpuGroup},
    error::{Error, Result},
};
use std::{fs, io};

#[derive(Copy, Clone)]
pub struct AffinityManager {
    ncpu: usize,
}

impl AffinityManager {
    pub fn new() -> Result<Self> {
        let mut ncpu = 0usize;
        for entry in fs::read_dir("/sys/devices/system/cpu")? {
            if let Ok(file_name) = entry?.file_name().into_string()
                && let Some((prefix, cpunum)) = file_name.split_once("cpu")
                && prefix.is_empty()
                && let Ok(cpunum) = str::parse::<usize>(cpunum)
                && cpunum > ncpu
            {
                ncpu = cpunum;
            }
        }

        Ok(Self { ncpu })
    }

    pub fn set_affinity(self, pid: usize, mask: &CpuSet) -> Result<()> {
        for entry in fs::read_dir(format!("/proc/{pid}/task"))? {
            if let Ok(file_name) = entry?.file_name().into_string()
                && let Ok(pid) = str::parse::<usize>(&file_name)
                && sched_getaffinity(self, pid)? != *mask
            {
                sched_setaffinity(self, pid, mask)?;
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn mock_new(ncpu: usize) -> Self {
        Self { ncpu }
    }
}

#[derive(Debug, PartialEq)]
pub struct CpuSet(Vec<u64>);

impl CpuSet {
    fn new(affinity_manager: AffinityManager) -> Self {
        Self(vec![0u64; affinity_manager.ncpu.div_ceil(64)])
    }

    #[cfg(test)]
    pub fn mock_new(ncpu: usize) -> Self {
        Self(vec![0u64; ncpu.div_ceil(64)])
    }

    pub fn from_cpu_group(affinity_manager: AffinityManager, cpu_group: &CpuGroup) -> Result<Self> {
        let mut cpu_set = Self::new(affinity_manager);
        for cpu in cpu_group.cpus() {
            match cpu {
                Cpu::Single(id) => cpu_set.set(affinity_manager, *id)?,
                Cpu::Range(ids) => {
                    for id in ids.clone() {
                        cpu_set.set(affinity_manager, id)?;
                    }
                }
            }
        }
        Ok(cpu_set)
    }

    #[cfg(test)]
    pub fn mock_set(&mut self, id: usize) {
        self.0[id / 64] |= 1 << (id % 64)
    }

    fn set(&mut self, affinity_manager: AffinityManager, id: usize) -> Result<()> {
        if id > affinity_manager.ncpu {
            Err(Error::CpuIndexTooHigh)
        } else {
            self.0[id / 64] |= 1 << (id % 64);
            Ok(())
        }
    }

    fn bytes(&self) -> usize {
        self.0.len() * size_of::<u64>()
    }

    unsafe fn as_const_ptr(&self) -> *const u64 {
        &raw const self.0[0]
    }

    unsafe fn as_mut_ptr(&mut self) -> *mut u64 {
        &raw mut self.0[0]
    }
}

fn sched_getaffinity(affinity_manager: AffinityManager, pid: usize) -> Result<CpuSet> {
    let mut mask = CpuSet::new(affinity_manager);
    let ret = unsafe {
        sched_libc::sched_getaffinity(pid as i32, mask.bytes() as u64, mask.as_mut_ptr())
    };

    match ret {
        0 => Ok(mask),
        _ => Err(Error::Io(io::Error::last_os_error())),
    }
}

fn sched_setaffinity(affinity_manager: AffinityManager, pid: usize, mask: &CpuSet) -> Result<()> {
    let ret = unsafe {
        sched_libc::sched_setaffinity(pid as i32, mask.bytes() as u64, mask.as_const_ptr())
    };

    match ret {
        0 => {
            let new_mask = sched_getaffinity(affinity_manager, pid)?;
            if new_mask != *mask {
                Err(Error::KernelIgnoredSchedSetAffinity)
            } else {
                Ok(())
            }
        }
        _ => Err(Error::Io(io::Error::last_os_error())),
    }
}

mod sched_libc {
    unsafe extern "C" {
        pub fn sched_getaffinity(pid: i32, cpusetsize: u64, mask: *mut u64) -> i32;
        pub fn sched_setaffinity(pid: i32, cpusetsize: u64, mask: *const u64) -> i32;
    }
}

#[cfg(test)]
mod tests {
    use crate::sched_affinity::CpuSet;

    #[test]
    fn cpuset_set_lowest() {
        let mut cpuset = CpuSet::mock_new(256);
        let mut expected = [0u64; 4];

        cpuset.mock_set(0);
        expected[0] = 0b1;

        assert_eq!(cpuset.0, expected);
    }

    #[test]
    fn cpuset_set_highest() {
        let mut cpuset = CpuSet::mock_new(256);
        let mut expected = [0u64; 4];

        cpuset.mock_set(127);
        expected[1] = 0b1 << 63;

        assert_eq!(cpuset.0, expected);
    }

    #[test]
    fn cpuset_set_boundary_highest() {
        let mut cpuset = CpuSet::mock_new(256);
        let mut expected = [0u64; 4];

        cpuset.mock_set(63);
        expected[0] = 0b1 << 63;

        assert_eq!(cpuset.0, expected);
    }

    #[test]
    fn cpuset_set_boundary_lowest() {
        let mut cpuset = CpuSet::mock_new(256);
        let mut expected = [0u64; 4];

        cpuset.mock_set(64);
        expected[1] = 0b1;

        assert_eq!(cpuset.0, expected);
    }

    #[test]
    fn cpuset_double_set() {
        let mut cpuset = CpuSet::mock_new(256);
        let mut expected = [0u64; 4];

        cpuset.mock_set(0);
        cpuset.mock_set(0);
        expected[0] = 0b1;

        assert_eq!(cpuset.0, expected);
    }
}
