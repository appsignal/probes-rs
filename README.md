# Probes

Rust library to read out system stats from a machine running Unix.

## Stats we want to support

### System wide stats

* [x] load (1, 5, 15)
* [x] cpu (user, nice, system, idle...)
* [x] mem (total, resident, virtual)
* [x] net (in/out in bytes/ops/packets)
* [ ] io (in/out in bytes/ops)
* [x] disk (drives in abs/rel)

### Per process stats

* [ ] cpu (user, nice, system, idle...)
* [x] mem (total, resident, virtual)
* [ ] net (in/out in bytes/ops/packets)
* [ ] io (in/out in bytes/ops)


# Contributing

## Setup

* Download and install [Vagrant](https://www.vagrantup.com/)
* Install/start the virtual machine by running `vagrant up`
* SSH to the vagrant machine by running `vagrant ssh`
* Navigate to the probes folder `cd /vagrant`
* Install rust nightly `multirust override nightly`
* Run the tests `cargo test`
* Add awesome features!
