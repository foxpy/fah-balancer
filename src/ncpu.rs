// Copyright Murad Karammaev
// SPDX-License-Identifier: MIT

use std::{fs, sync::OnceLock, io};

static NCPU: OnceLock<usize> = OnceLock::new();

pub fn ncpu() -> usize {
    *NCPU.get_or_init(|| match count_cpus() {
        Ok(ncpu) => ncpu,
        Err(e) => panic!("failed to count CPUs: {e}"),
    })
}

fn count_cpus() -> io::Result<usize> {
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

    Ok(ncpu)
}
