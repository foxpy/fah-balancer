// Copyright Murad Karammaev
// SPDX-License-Identifier: MIT

mod arg;
mod error;
mod fah;
mod sched_affinity;

use error::{Error, Result};
use std::{process, thread, time};

const SLEEP_DURATION: time::Duration = time::Duration::from_secs(10);

fn main() {
    let cpu_groups = match arg::Arg::parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Failed to parse CPU groups: {e}");
            process::exit(1);
        }
    }
    .cpu_groups;

    let affinity_manager = match sched_affinity::AffinityManager::new() {
        Ok(affinity_manager) => affinity_manager,
        Err(e) => {
            eprintln!("Failed to create affinity manager: {e}");
            process::exit(1);
        }
    };

    loop {
        match fah::FahClient::find() {
            Ok(Some(fah_client)) => {
                let err = main_loop(affinity_manager, fah_client, &cpu_groups);
                eprintln!(
                    "error: {err}. Will try again in {} seconds",
                    SLEEP_DURATION.as_secs_f64()
                );
                thread::sleep(SLEEP_DURATION);
            }
            Ok(None) => {
                eprintln!(
                    "error: fah-client not found. Will try again in {} seconds",
                    SLEEP_DURATION.as_secs_f64()
                );
                thread::sleep(SLEEP_DURATION);
            }
            Err(err) => {
                eprintln!(
                    "error: failed to find fah-client: {err}. Will try again in {} seconds",
                    SLEEP_DURATION.as_secs_f64()
                );
                thread::sleep(SLEEP_DURATION);
            }
        }
    }
}

fn main_loop(
    affinity_manager: sched_affinity::AffinityManager,
    fah_client: fah::FahClient,
    cpu_groups: &[arg::CpuGroup],
) -> Error {
    loop {
        let fah_cores = match fah_client.cores() {
            Ok(fah_cores) => fah_cores,
            Err(e) => return e,
        };

        let commands = match schedule(affinity_manager, fah_cores, cpu_groups.to_vec()) {
            Ok(commands) => commands,
            Err(e) => return e,
        };

        for (pid, mask) in commands {
            if let Err(e) = affinity_manager.set_affinity(pid, &mask) {
                return e;
            }
        }

        thread::sleep(SLEEP_DURATION);
    }
}

fn schedule(
    affinity_manager: sched_affinity::AffinityManager,
    mut fah_cores: Vec<fah::FahCore>,
    mut cpu_groups: Vec<arg::CpuGroup>,
) -> Result<Vec<(usize, sched_affinity::CpuSet)>> {
    let mut commands = vec![];

    while let Some(fah_core) = fah_cores.pop() {
        if let Some(biggest_cpu_group) = cpu_groups.last_mut() {
            if fah_core.threads > biggest_cpu_group.total_cpus {
                return Err(Error::OutOfCpuGroups);
            }

            commands.push((
                fah_core.pid,
                sched_affinity::CpuSet::from_cpu_group(affinity_manager, biggest_cpu_group)?,
            ));
            biggest_cpu_group.total_cpus -= fah_core.threads;
        } else {
            return Err(Error::OutOfCpuGroups);
        }

        if let Some(biggest_cpu_group) = cpu_groups.last()
            && biggest_cpu_group.total_cpus == 0
        {
            cpu_groups.pop();
        }

        // CPU groups with more total_cpus go last
        cpu_groups.sort_by(|a, b| a.total_cpus.cmp(&b.total_cpus));
    }

    Ok(commands)
}

#[cfg(test)]
mod tests {
    use crate::{arg, error::Error, fah, sched_affinity};

    fn cores(cores: &[usize]) -> Vec<fah::FahCore> {
        cores
            .iter()
            .cloned()
            .enumerate()
            .map(|(pid, threads)| fah::FahCore {
                pid: pid + 1,
                threads,
            })
            .collect()
    }

    fn groups(groups: &[&str]) -> Vec<arg::CpuGroup> {
        groups
            .iter()
            .map(|group| arg::CpuGroup::try_from(*group).unwrap())
            .collect()
    }

    fn cpus(ids: &[usize]) -> sched_affinity::CpuSet {
        let mut cpuset = sched_affinity::CpuSet::mock_new(256);
        for id in ids {
            cpuset.mock_set(*id);
        }
        cpuset
    }

    fn assert_sequential_pids(entries: &[(usize, sched_affinity::CpuSet)]) {
        for (i, (pid, _)) in entries.iter().enumerate() {
            assert_eq!(*pid, i + 1);
        }
    }

    fn am() -> sched_affinity::AffinityManager {
        sched_affinity::AffinityManager::mock_new(256)
    }

    fn cmp(
        mut actual: Vec<(usize, sched_affinity::CpuSet)>,
        mut expected: Vec<(usize, sched_affinity::CpuSet)>,
    ) {
        actual.sort_by_key(|(pid, _)| *pid);
        assert_sequential_pids(&actual);
        expected.sort_by_key(|(pid, _)| *pid);
        assert_sequential_pids(&expected);
        assert_eq!(actual, expected);
    }

    #[test]
    fn one_group_one_core() {
        cmp(
            super::schedule(am(), cores(&[2]), groups(&["0,1"])).unwrap(),
            vec![(1, cpus(&[0, 1]))],
        );
    }

    #[test]
    fn group_bigger_than_core() {
        cmp(
            super::schedule(am(), cores(&[2]), groups(&["0-3"])).unwrap(),
            vec![(1, cpus(&[0, 1, 2, 3]))],
        );
    }

    #[test]
    fn core_bigger_than_group() {
        let err = super::schedule(am(), cores(&[10]), groups(&["0-7"])).unwrap_err();
        assert!(matches!(err, Error::OutOfCpuGroups));
    }

    #[test]
    fn two_groups_two_cores() {
        cmp(
            super::schedule(am(), cores(&[2, 4]), groups(&["0,1", "8-11"])).unwrap(),
            vec![(1, cpus(&[0, 1])), (2, cpus(&[8, 9, 10, 11]))],
        );
    }

    #[test]
    fn two_cores_larger_than_group() {
        let err = super::schedule(am(), cores(&[4, 6]), groups(&["0-7"])).unwrap_err();
        assert!(matches!(err, Error::OutOfCpuGroups));
    }

    #[test]
    fn large_group_with_two_cores() {
        cmp(
            super::schedule(am(), cores(&[2, 4]), groups(&["0-5"])).unwrap(),
            vec![
                (1, cpus(&[0, 1, 2, 3, 4, 5])),
                (2, cpus(&[0, 1, 2, 3, 4, 5])),
            ],
        );
    }
}
