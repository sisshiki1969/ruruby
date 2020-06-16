use super::vm_inst::Inst;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct PerfCounter {
    pub count: u64,
    pub duration: Duration,
}

impl PerfCounter {
    pub fn new() -> Self {
        PerfCounter {
            count: 0,
            duration: Duration::from_secs(0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Perf {
    counter: Vec<PerfCounter>,
    timer: Instant,
    prev_inst: u8,
}

impl Perf {
    pub const GC: u8 = 252;
    pub const CODEGEN: u8 = 253;
    pub const EXTERN: u8 = 254;
    pub const INVALID: u8 = 255;
}

impl Perf {
    pub fn new() -> Self {
        Perf {
            counter: vec![PerfCounter::new(); 256],
            timer: Instant::now(),
            prev_inst: Perf::INVALID,
        }
    }

    pub fn add(&mut self, other: &Perf) {
        for i in 0..256 {
            self.counter[i].count += other.counter[i].count;
            self.counter[i].duration += other.counter[i].duration;
        }
    }

    /// Record duration for current instruction.
    pub fn get_perf(&mut self, next_inst: u8) {
        let prev = self.prev_inst;
        if prev != Perf::INVALID {
            self.counter[prev as usize].count += 1;
            self.counter[prev as usize].duration += self.timer.elapsed();
        }
        self.timer = Instant::now();
        self.prev_inst = next_inst;
    }

    pub fn get_perf_no_count(&mut self, next_inst: u8) {
        self.get_perf(next_inst);
        if next_inst != Perf::INVALID {
            self.counter[next_inst as usize].count -= 1;
        }
    }

    pub fn set_prev_inst(&mut self, inst: u8) {
        self.prev_inst = inst;
    }

    pub fn get_prev_inst(&mut self) -> u8 {
        self.prev_inst
    }

    pub fn print_perf(&self) {
        eprintln!("Performance analysis for Inst:");
        eprintln!("------------------------------------------");
        eprintln!(
            "{:<13} {:>10} {:>8} {:>8}",
            "Inst name", "count", "%time", "nsec"
        );
        eprintln!("{:<13} {:>10} {:>8} {:>8}", "", "", "", "/inst");
        eprintln!("------------------------------------------");
        let mut sum = std::time::Duration::from_secs(0);
        for c in &self.counter {
            sum += c.duration;
        }
        for (
            i,
            PerfCounter {
                count: c,
                duration: d,
            },
        ) in self.counter.iter().enumerate()
        {
            if *c == 0 {
                continue;
            }
            eprintln!(
                "{:<13} {:>10} {:>8.2} {:>8}",
                if i as u8 == Perf::CODEGEN {
                    "CODEGEN"
                } else if i as u8 == Perf::EXTERN {
                    "EXTERN"
                } else if i as u8 == Perf::GC {
                    "GC"
                } else {
                    Inst::inst_name(i as u8)
                },
                if *c > 10000_000 {
                    format!("{:>9}M", c / 1000_000)
                } else if *c > 10000 {
                    format!("{:>9}K", c / 1000)
                } else {
                    format!("{:>10}", *c)
                },
                (d.as_micros() as f64) * 100.0 / (sum.as_micros() as f64),
                d.as_nanos() / (*c as u128)
            );
        }
        eprintln!("------------------------------------------");
    }
}
