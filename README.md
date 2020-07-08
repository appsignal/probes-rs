# Probes

[![Build Status](https://travis-ci.org/appsignal/probes-rs.svg?branch=main)](https://travis-ci.org/appsignal/probes-rs)
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

## Contributing

Thinking of contributing to our Probes package? Awesome! 🚀

Please follow our [Contributing guide][contributing-guide] in our
documentation and follow our [Code of Conduct][coc].

Also, we would be very happy to send you Stroopwafles. Have look at everyone
we send a package to so far on our [Stroopwafles page][waffles-page].

## Setup

* Download and install [Docker](https://www.docker.com/)
* Build the images: `make build`
* Make sure that the path where this code resided can be mounted as
  a volume with Docker.
* Run the tests on all images: `make test`
* Add awesome features!

The tests on Travis are only run directly on that VM. Make sure to run
the full test suite manually before every release.

[contributing-guide]: http://docs.appsignal.com/appsignal/contributing.html
[coc]: https://docs.appsignal.com/appsignal/code-of-conduct.html
[waffles-page]: https://appsignal.com/waffles
