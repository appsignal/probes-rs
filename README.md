# Probes

Rust library to read out system stats from a machine running Unix.

## Stats we want to support

### System wide stats

- load (1, 5, 15)
- cpu (user, nice, system, idle...)
- mem (total, resident, virtual)
- net (in/out in bytes/ops/packets)
- io (in/out in bytes/ops)
- disk (drives in abs/rel)

### Per process stats

- cpu (user, nice, system, idle...)
- mem (total, resident, virtual)
- net (in/out in bytes/ops/packets)
- io (in/out in bytes/ops)
