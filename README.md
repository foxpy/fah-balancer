# fah-balancer

Define CPU groups and let the balancer assign each FAH CPU core to a corresponding group. Useful for:
- systems with heterogeneous CPU architectures, like most consumer ARM CPUs, Intel's P and E cores or AMD's Zen 5c;
- systems with SMT;
- systems with more than 64 CPUs.

This tool should primarily be used on systems with heterogeneous architectures because of the way GROMACS schedules work across threads. It is preferable to run all threads of a single FAH CPU core on CPUs of similar speed, because simulation speed is ultimately as quick as the slowest CPU. Consider following scenario: FAH CPU core runs with 10 threads on a system with 10 CPUs, out of which 8 CPUs are high performance and 2 CPUs are low power. Let's imagine that low power CPUs are 3 times slower than high performance ones. That means, running 10 threads on all CPUs is roughly `8*3/10 = 2.4` times slower than running 8 threads on 8 high performance CPUs.

On systems with SMT, this issue doesn't exist, but you might run into quality of life issues, consider a following scenario: a desktop PC user has 4 SMT CPUs (8 threads total) and wants to donate 75% of processing power to FAH. Since the base load of FAH is not less than 50% of all available threads, that means user tasks will always share a power hungry SMT sibling, which might make user tasks anywhere from slightly slower to 50% slower (or maybe even more?). In order to avoid that, user could schedule FAH core to always use 3 physical CPUs (and their 6 SMT threads), while always having a single physical CPU available for their interactive tasks.

Finally, this tool might be useful on systems with more than 64 CPUs, but this is only theoretical. I can think of following scenarios with a probability of this tool being useful:
- some Intel Xeons have a "priority CPU group" feature which clocks some CPUs slightly faster than the others. While it is technically not a heterogeneous architecture, it practically still behaves like one;
- some CPUs have non-uniform distributed caches, for example AMD has separate L3 slices per CCD or Intel has separate L2 slices per E-core cluster, and so on. Maybe there is some potential for performance improvement using some smart scheduling in these situations, but I never measured it;
- on systems with multiple sockets, accessing different chunks of memory can be more or less expensive depending on which CPU the task currently runs on. I have no practical knowledge of whether Linux is capable of correctly scheduling FAH cores on NUMA systems or not, and I have never measured actual performance in these scenarios;
- systems with more than 64 CPUs in general might require you to run more than a single FAH core, because [GROMACS doesn't scale well beyond 64 threads](https://forum.foldingathome.org/viewtopic.php?p=369744#p369744). While I would expect an OS scheduler to do a good job in general, maybe using this tool will improve (or not) overall performance via limiting the amount of work CPU scheduler has to do balancing all these threads and potentially improving cache locality for each FAH core instance.


### How to use

First, you have to specify CPU groups manually. This tool cannot do that for you and will refuse to run if you don't give it any CPU groups. You are probably going to use this tool on a heterogeneous CPU and you might be wondering, which CPUs to select. This is very easy to do using a provided `cpu_speed.sh` script. It collects random from `/dev/urandom` on each CPU available in your system and outputs speed in megabytes per second. Make sure you don't have any background tasks running and execute the script. The output might look like something like this:

```
cpu   0:    684.643 MB/s
cpu   1:    698.454 MB/s
cpu   2:    686.704 MB/s
cpu   3:    688.333 MB/s
cpu   4:    678.575 MB/s
cpu   5:    688.217 MB/s
cpu   6:    683.564 MB/s
cpu   7:    699.127 MB/s
cpu   8:    696.534 MB/s
cpu   9:    696.474 MB/s
cpu  10:    690.405 MB/s
cpu  11:    697.167 MB/s
cpu  12:    387.793 MB/s
cpu  13:    387.548 MB/s
cpu  14:    387.296 MB/s
cpu  15:    387.637 MB/s
cpu  16:    387.171 MB/s
cpu  17:    386.495 MB/s
cpu  18:    388.215 MB/s
cpu  19:    387.998 MB/s
```

This example output was collected on i5-14500, a CPU with 6 P cores and 8 E cores. As you can see, P cores are almost two times faster than E cores (well, actually, SMT in this case makes it harder to eastimate performance difference correctly, but still, it is easy to see that we have a group of 12 CPUs which are considerably more powerful than a second group of 8 CPUs). That means, we can afford running two FAH CPU cores using two CPU groups: a core with 12 threads will run on CPU group 0-11 and a core with 8 threads will run on a group 12-19. Unfortunately, OS schedulers, while being smart, aren't smart enough to understand what's going on inside GROMACS and how to schedule threads correctly in this situation. To fix this, we will have to run `fah-balancer` like this:

```bash
./fah-balancer 0-11 12-19
```

This tells `fah-balancer` that we want it to isolate FAH cores from each other in two different CPU groups. It will periodically monitor for running FAH cores and automatically assign correct CPU affinity to them. While there is technically no need to setup strictly two FAH cores with 12 and 8 threads and things will work just fine if one sets up FAH cores to use less threads than CPU groups have, things will break if one breaks one of the following conditions:
- there is a FAH core instance with more threads than in the biggest CPU group;
- total number of FAH core threads exceeds total number of threads in all CPU groups.

Also keep in mind that it is better not to have more FAH core instances than there are CPU groups. The scheduling algorithm in `fah-balancer` will try to make it work, but it is not very smart and will not always succeed.

### Installation

There are no external dependencies to this software. It is enough to have just the Rust toolchain installed. Simply run:

```bash
make release
```

This will produce a statically linked binary at `target/x86_64-unknown-linux-musl/release/fah-balancer`, which you could then copy anywhere you want to, with `/usr/local/bin/` being a sensible default.

You might as well want to consider installing it as a systemd service. For that, copy `fah-balancer.service` to `/etc/systemd/system/` and edit it to provide your CPU groups of choice. Finally, run `systemctl daemon-reload` and `systemctl enable --now fah-balancer.service`.

`fah-balancer` will start automatically after `fah-client` is started and it will immediately begin scheduling CPU cores.
