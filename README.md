# Probes

[![Build Status](https://travis-ci.org/appsignal/probes-rs.svg?branch=master)](https://travis-ci.org/appsignal/probes-rs)
[![Crate](http://meritbadge.herokuapp.com/probes)](https://crates.io/crates/probes)

Rust library to read out system stats from a machine running Unix.
Currently only supports Linux.

## Supported stats

### System wide

* load
* cpu
* memory
* network
* io
* disk

### Per process

* memory (total, resident, virtual)

# Contributing

Pull requests welcome!

## Setup

* Download and install [Docker](https://www.docker.com/)
* Build the images: `make build`
* Make sure that the path where this code resided can be mounted as
  a volume with Docker.
* Run the tests on all images: `make test`
* Add awesome features!

The tests on Travis are only run directly on that VM. Make sure to run
the full test suite manually before every release.
