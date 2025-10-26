use rand::prelude::*;
use hashbrown::HashMap;
use std::fs::File;
use std::io::Write;

const ORG_SIZE: usize = 64;
const POP_SIZE: usize = 1024;
const STEP_LIMIT: usize = 2048;
const EPOCHS: usize = 2_000_000;

// try lowering this later to let species persist longer
const MUTATION_RATE: f64 = 0.0002;

// how big a takeover before we consider it "interesting"
const REPLICATOR_THRESHOLD: usize = 100;

type Org = [u8; ORG_SIZE];

fn main() {
    let mut rng = thread_rng();

    let mut population: Vec<Org> = (0..POP_SIZE)
        .map(|_| {
            let mut o = [0u8; ORG_SIZE];
            rng.fill_bytes(&mut o);
            o
        })
        .collect();

    for epoch in 0..EPOCHS {
        run_epoch(&mut population, &mut rng);
        mutate(&mut population, &mut rng, MUTATION_RATE);

        if epoch % 1000 == 0 {
            let stats = diversity_stats(&population);

            println!(
                "epoch {:7} unique_orgs={} max_count={}",
                epoch, stats.unique_cnt, stats.max_count
            );
            println!(
                "  dominant (first32 hex): {}",
                hex_prefix(&stats.dominant, 32)
            );

            if stats.max_count >= REPLICATOR_THRESHOLD {
                save_replicator_and_assay(epoch, &stats.dominant, &mut rng);
            }
        }
    }
}

/// If something took over hard enough, analyze it.
fn save_replicator_and_assay(epoch: usize, genome: &Org, rng: &mut ThreadRng) {
    // 1. dump the full 64-byte genome to a file (hex)
    let mut f = File::create(format!("replicator_epoch_{epoch}.hex")).unwrap();
    writeln!(f, "epoch {epoch}").unwrap();
    writeln!(f, "{}", hex_full(genome)).unwrap();

    // 2. replication assay:
    //    How often does genome overwrite a random victim into (exactly) itself?
    let trials = 1000;
    let rate_ab = exact_match_rate(genome, rng, trials, true);
    let rate_ba = exact_match_rate(genome, rng, trials, false);

    println!(
        "  [assay epoch {epoch}] infect_as_A->B success_rate={:.3}  infect_as_B->A success_rate={:.3}",
        rate_ab, rate_ba
    );
}

/// returns fraction [0..1] of cases where victim becomes EXACT copy of `template`
/// If as_a=true we do interact(template, victim)
/// else we do interact(victim, template)
fn exact_match_rate(template: &Org, rng: &mut ThreadRng, trials: usize, as_a: bool) -> f64 {
    let mut success = 0usize;
    for _ in 0..trials {
        // random victim each time
        let mut victim = [0u8; ORG_SIZE];
        rng.fill_bytes(&mut victim);

        let (out_a, out_b) = if as_a {
            interact(*template, victim)
        } else {
            interact(victim, *template)
        };

        // check if EITHER output half matches template
        if out_a == *template || out_b == *template {
            success += 1;
        }
    }
    success as f64 / trials as f64
}

/// shuffle, pair, interact
fn run_epoch(pop: &mut [Org], rng: &mut ThreadRng) {
    let mut idxs: Vec<usize> = (0..pop.len()).collect();
    idxs.shuffle(rng);

    for pair in idxs.chunks_exact(2) {
        let i = pair[0];
        let j = pair[1];

        let a = pop[i];
        let b = pop[j];

        let (na, nb) = interact(a, b);
        pop[i] = na;
        pop[j] = nb;
    }
}

/// combine two orgs, run VM, split
fn interact(a: Org, b: Org) -> (Org, Org) {
    let mut tape = [0u8; ORG_SIZE * 2];
    tape[..ORG_SIZE].copy_from_slice(&a);
    tape[ORG_SIZE..].copy_from_slice(&b);

    run_bff(&mut tape);

    let mut out_a = [0u8; ORG_SIZE];
    let mut out_b = [0u8; ORG_SIZE];
    out_a.copy_from_slice(&tape[..ORG_SIZE]);
    out_b.copy_from_slice(&tape[ORG_SIZE..]);

    (out_a, out_b)
}

/// radiation
fn mutate(pop: &mut [Org], rng: &mut ThreadRng, prob_per_byte: f64) {
    for org in pop.iter_mut() {
        for byte in org.iter_mut() {
            if rng.gen_bool(prob_per_byte) {
                *byte = rng.gen();
            }
        }
    }
}

struct PopStats {
    unique_cnt: usize,
    max_count: usize,
    dominant: Org,
}

/// count unique genomes + dominant genome
fn diversity_stats(pop: &[Org]) -> PopStats {
    let mut counts: HashMap<&Org, usize> = HashMap::new();
    for org in pop {
        *counts.entry(org).or_insert(0) += 1;
    }

    let mut best_org: Option<&Org> = None;
    let mut best_count = 0usize;
    for (org, c) in counts.iter() {
        if *c > best_count {
            best_count = *c;
            best_org = Some(org);
        }
    }

    let dom = if let Some(o) = best_org {
        *o
    } else {
        [0u8; ORG_SIZE]
    };

    PopStats {
        unique_cnt: counts.len(),
        max_count: best_count,
        dominant: dom,
    }
}

/// hex helpers
fn hex_prefix(org: &Org, n: usize) -> String {
    let mut s = String::new();
    let upto = n.min(org.len());
    for b in &org[..upto] {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

fn hex_full(org: &Org) -> String {
    hex_prefix(org, org.len())
}

/// VM
fn run_bff(tape: &mut [u8]) {
    let len = tape.len();
    let mut ip: usize = 0;
    let mut head0: usize = 0;
    let mut head1: usize = len / 2;

    let mut steps = 0;
    while steps < STEP_LIMIT {
        if ip >= len {
            break;
        }

        let inst = tape[ip];
        match inst {
            b'>' => {
                head0 = (head0 + 1) % len;
                ip += 1;
            }
            b'<' => {
                head0 = (head0 + len - 1) % len;
                ip += 1;
            }
            b'}' => {
                head1 = (head1 + 1) % len;
                ip += 1;
            }
            b'{' => {
                head1 = (head1 + len - 1) % len;
                ip += 1;
            }
            b'+' => {
                tape[head0] = tape[head0].wrapping_add(1);
                ip += 1;
            }
            b'-' => {
                tape[head0] = tape[head0].wrapping_sub(1);
                ip += 1;
            }
            b'.' => {
                tape[head1] = tape[head0];
                ip += 1;
            }
            b',' => {
                tape[head0] = tape[head1];
                ip += 1;
            }
            b'[' => {
                if tape[head0] == 0 {
                    let mut depth = 1usize;
                    let mut pos = ip + 1;
                    let mut found = false;
                    while pos < len {
                        match tape[pos] {
                            b'[' => depth += 1,
                            b']' => {
                                depth -= 1;
                                if depth == 0 {
                                    ip = pos + 1;
                                    found = true;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        pos += 1;
                    }
                    if !found {
                        break;
                    }
                } else {
                    ip += 1;
                }
            }
            b']' => {
                if tape[head0] != 0 {
                    if ip == 0 {
                        break;
                    }
                    let mut depth = 1usize;
                    let mut pos = ip - 1;
                    let mut found = false;
                    loop {
                        match tape[pos] {
                            b']' => depth += 1,
                            b'[' => {
                                depth -= 1;
                                if depth == 0 {
                                    ip = pos + 1;
                                    found = true;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        if pos == 0 {
                            break;
                        }
                        pos -= 1;
                    }
                    if !found {
                        break;
                    }
                } else {
                    ip += 1;
                }
            }
            _ => {
                ip += 1;
            }
        }

        steps += 1;
    }
}
