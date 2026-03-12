#!/usr/bin/env bash

# Copyright Murad Karammaev
# SPDX-License-Identifier: MIT

NBYTES="$(python3 -c "print(5 * 10**8)")"

set -euo pipefail
IFS=$'\n\t'

for i in $(seq 0 "$(( $(nproc) - 1 ))"); do
	time_before="$(date '+%s%N')"
	taskset -c "$i" head -c "$NBYTES" /dev/urandom >/dev/null
	time_after="$(date '+%s%N')"
	diff="$(python3 -c "print($time_after - $time_before)")"
	speed="$(python3 -c "print(\"{:10.3f}\".format($NBYTES / $diff * 1e3))")"
	printf 'cpu %3d: ' "$i"
	echo "$speed MB/s"
done
