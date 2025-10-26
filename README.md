

[soup.webm](https://github.com/user-attachments/assets/278bdbb3-db7b-4979-99ef-767a635cc438)


# Soup: Digital Abiogenesis in Rust

This repo contains a minimal reproduction of the experiments described in “[Spontaneous Emergence of Digital Life from Random Programs]” ([arXiv:2406.19108](https://arxiv.org/pdf/2406.19108)).  The authors show that if you place a population of random programs in a simple computational substrate and allow them to interact by executing their concatenated code on shared memory, self-replicating “organisms” can spontaneously arise and evolve without any explicit fitness function or reproduction mechanism.

## What this code does

The core of this project is a tiny virtual machine and a “primordial soup” loop:

- **Organisms** are fixed-length byte arrays (64 bytes by default).  They serve as both code and data.
- Each epoch, we randomly pair organisms, concatenate them into a single 128 byte tape and run a Brainfuck like interpreter on that tape.  The interpreter operates with two data pointers (`head0` and `head1`) and supports instructions for moving pointers, incrementing/decrementing bytes, copying bytes between heads, and looping.  Unrecognized bytes are treated as no ops.  Execution halts after a fixed number of steps or on malformed loops.
- After execution, the 128 byte tape is split back into two 64 byte organisms.  An optional per‟byte mutation rate applies random flips, mimicking cosmic radiation.
- We repeat this process for many epochs, tracking how many unique genomes exist and how large the most common genome is.  When a self‟replicating organism emerges, it overwrites its neighbours into copies of itself and the population diversity collapses.  Later, mutation drifts can destroy the lineage and the soup returns to noise.

## Modifications and results

While the paper used a variety of substrates, this Rust implementation makes a few choices that dramatically improve the likelihood of spontaneous replication:

- **Two‟headed copying:** We start `head0` at the beginning of the first organism and `head1` at the start of the second half.  This means that the instructions `.` and `,` copy bytes between organisms rather than within a single organism.  Without this change, most random programs only smear data within one half and rarely assemble a working copier.
- **Mutation:** We add a small mutation rate (default `0.0002` per byte per epoch).  Mutation is not strictly necessary for abiogenesis, but it accelerates the exploration of program space and allows lineages to evolve.
- **Metrics:** The program reports `unique_orgs` (the number of distinct genomes in the population) and `max_count` (how many copies of the dominant genome exist).  When `max_count` jumps and `unique_orgs` collapses, a replicator has appeared.

### Observed behaviour

Running the program with `POP_SIZE=1024`, `ORG_SIZE=64`, `STEP_LIMIT=2048`, `EPOCHS=100000` and `MUTATION_RATE=0.0002` on a laptop produced multiple independent origin-of-life events.  Initially `unique_orgs` stays equal to the population size (all organisms are distinct).  After tens of thousands of epochs, a replicator appears and quickly dominates:

```
epoch   13000 unique_orgs=85   max_count=101
... self replicator arises (replication success rate ~100 %)
epoch   14000 unique_orgs=217  max_count=101
epoch   15000 unique_orgs=193  max_count=74
...
epoch   24000 unique_orgs=836  max_count=32
...
epoch   34000 unique_orgs=98   max_count=147
...
epoch   57000 unique_orgs=1024 max_count=1    ← lineage collapses back to noise
...
epoch   67000 unique_orgs=333  max_count=148  ← new lineage arises
```

Whenever `max_count` exceeds a threshold (100 by default), the program performs a replication assay: it pairs the dominant genome with random victims and reports the probability that the victim becomes an exact copy.  Successful replicators consistently show `infect_as_A->B success_rate ≈ 1.0` and `infect_as_B->A ≈ 0.2`, demonstrating that they aggressively copy themselves into neighbours.

These cycles of abiogenesis, ecological dynamics and extinction match the qualitative behaviour reported in the paper.

## Building and running

You will need a recent Rust toolchain.  To run the default experiment:

```bash
git clone https://github.com/samoylenkodmitry/soup.git
cd soup
cargo run --release
```

The constants `ORG_SIZE`, `POP_SIZE`, `STEP_LIMIT`, `EPOCHS` and `MUTATION_RATE` at the top of `src/main.rs` control the simulation.  Increasing the population size and step limit will increase the probability of spontaneous replication but require more CPU time.

## Further reading

- The original paper: **Spontaneous Emergence of Digital Life from Random Programs** by Paul W. Florensky et al., 2024.  It explores several computational substrates and demonstrates that digital life emerges robustly without explicit fitness or selection.
- [A minimal Rust implementation](https://github.com/samoylenkodmitry/soup) (this repo).
