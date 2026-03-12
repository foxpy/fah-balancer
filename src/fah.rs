// Copyright Murad Karammaev
// SPDX-License-Identifier: MIT

use crate::error::Result;
use std::fs;

const KNOWN_CORES: &[&str] = &["FahCore_a8", "FahCore_a9"];

pub struct FahClient {
    pid: usize,
}

#[derive(Debug)]
pub struct FahCore {
    pub pid: usize,
    pub threads: usize,
}

impl FahClient {
    pub fn find() -> Result<Option<Self>> {
        for entry in fs::read_dir("/proc")? {
            if let Ok(file_name) = entry?.file_name().into_string()
                && let Ok(pid) = str::parse::<usize>(&file_name)
                && let Ok(cmdline) = fs::read_to_string(format!("/proc/{pid}/cmdline"))
                && let Some((basename, _)) = extract_basename(&cmdline)
                && basename == "fah-client"
            {
                return Ok(Some(Self { pid }));
            }
        }

        Ok(None)
    }

    pub fn cores(&self) -> Result<Vec<FahCore>> {
        let mut cores = vec![];

        for child_pid in fs::read_to_string(format!("/proc/{0}/task/{0}/children", self.pid))?
            .trim()
            .split(' ')
        {
            if let Ok(pid) = str::parse::<usize>(child_pid)
                && let Ok(cmdline) = fs::read_to_string(format!("/proc/{pid}/cmdline"))
                && let Some((basename, args)) = extract_basename(&cmdline)
                && KNOWN_CORES.contains(&basename)
                && let Some(threads) = args.skip_while(|&arg| arg != "-np").nth(1)
                && let Ok(threads) = str::parse::<usize>(threads)
            {
                cores.push(FahCore { pid, threads });
            }
        }

        // cores with more threads go last
        cores.sort_by(|a, b| a.threads.cmp(&b.threads));

        Ok(cores)
    }
}

fn extract_basename<'a>(cmdline: &'a str) -> Option<(&'a str, std::str::Split<'a, char>)> {
    let mut cmdline = cmdline.split('\0');
    if let Some(program_path) = cmdline.next()
        && let Some(basename) = program_path.split('/').next_back()
    {
        Some((basename, cmdline))
    } else {
        None
    }
}
