# Probes

[![Build Status](https://travis-ci.org/appsignal/probes-rs.svg?branch=master)](https://travis-ci.org/appsignal/probes-rs)
[![Crate](http://meritbadge.herokuapp.com/sql_lexer)](https://crates.io/crates/probes)

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

* Download and install [Vagrant](https://www.vagrantup.com/)
* Install/start the virtual machine by running `vagrant up`
* SSH to the vagrant machine by running `vagrant ssh`
* Navigate to the probes folder `cd /vagrant`
* Install rust nightly `multirust override nightly`
* Run the tests `cargo test`
* Add awesome features!
