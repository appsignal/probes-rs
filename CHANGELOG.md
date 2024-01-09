# Changelog

## 0.5.4

- Fix disk usage returning a Vec with no entries on Alpine Linux when the `df --local` command fails.

## 0.5.3

- Support disk usage reporting (using `df`) on Alpine Linux.
- When a disk mountpoint has no inodes usage percentage, skip the mountpoint, and report the inodes information successfully for the inodes that do have an inodes usage percentage.

## 0.5.2

- Normalize CPU usage percentages for cgroups v2 systems

## 0.5.1

- Add support for CPU total usage for `/proc` based systems (VMs).

## 0.5.0

- Add support for cgroups v2 in CPU and memory metrics
- Change memory metrics struct to allow for values being optional

## 0.4.3

- Add shared memory metric function `Memory.shmem()`.

## 0.4.2

- Support shared memory metric. For containers only cgroups v1 is supported.

## 0.2.0

* Initial support for containers
