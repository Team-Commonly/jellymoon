//! What a trial's child measures of its own reading and drawing: processor time and memory. A picture is
//! accepted only under a stated cost of each, because a picture that ends can still cost the page a second and
//! hundreds of megabytes at every draw. Neither number moves with how busy the machine is, as time on the wall
//! does, so a refusal for cost is an answer about the picture, and is kept.
//!
//! Memory is what the process holds at this moment, asked every [`EVERY`] and when the draw has ended, less
//! what it held before the picture was read: a peak between two asks that is given back before the next is not
//! seen. Processor time is the process's own clock where a system has one a program may ask without unsafe code
//! (every Unix); elsewhere time on the wall bounds it from above, and what is over by that measure is said to be
//! unsure, and is not kept.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use super::trial::Said;

/// The most processor time reading and drawing one picture may take. Measured on one Mac (`README.md`), in a
/// release and in a debug build: the heaviest picture the kit's tests know inside the rules takes 8 ms and
/// 551 ms, an ordinary one (`tests/ordinary.rs`) under 1 ms and 7 ms, and the lightest picture measured that the
/// page pays dearly for (`tests/ordinary.rs`, 30 pairs of curves) 381 ms and 9.5 s. So the most is 12 and 3.6
/// times the heaviest known inside the rules, and under that dear one by 3.8 and 4.7 times.
pub const MOST_PROCESSOR: Duration = Duration::from_millis(if cfg!(debug_assertions) { 2_000 } else { 100 });

/// The most memory reading and drawing one picture may take, over what the child held before a parser read it.
/// The heaviest picture known inside the rules takes 2.5 MiB in a release and 4.6 MiB in a debug build, most of
/// it the 512 by 512 pixels of the largest picture there may be; the dear one above takes 87 and 75 MiB. So the
/// most is 13 and 6.9 times the first, and under the second by 2.7 and 2.3 times.
pub const MOST_MEMORY: u64 = 32 * 1024 * 1024;

/// How often the cost is asked while the picture is drawn.
const EVERY: Duration = Duration::from_millis(2);

/// The memory this process holds now.
struct Held {
    system: System,
    pid: Pid,
}

impl Held {
    fn new() -> Held {
        Held { system: System::new(), pid: Pid::from_u32(std::process::id()) }
    }

    fn now(&mut self) -> Option<u64> {
        let only = ProcessesToUpdate::Some(&[self.pid]);
        self.system.refresh_processes_specifics(only, ProcessRefreshKind::new().with_memory());
        self.system.process(self.pid).map(sysinfo::Process::memory).filter(|held| *held > 0)
    }
}

/// The processor time this process has used, where the system says.
#[cfg(unix)]
fn processor() -> Option<Duration> {
    let used = rustix::time::clock_gettime(rustix::time::ClockId::ProcessCPUTime);
    Some(Duration::new(u64::try_from(used.tv_sec).ok()?, u32::try_from(used.tv_nsec).ok()?))
}

#[cfg(not(unix))]
fn processor() -> Option<Duration> {
    None
}

/// The cost of one picture, from [`Meter::start`] on.
pub(super) struct Meter {
    wall: Instant,
    processor: Option<Duration>,
    before: u64,
    peak: Arc<AtomicU64>,
    held: Held,
}

impl Meter {
    /// Starts measuring, before the picture is read by any parser. From here on `tell` is called, on a thread
    /// of the meter's own, as soon as a cost is passed: the draw is not waited for. An error is why this system
    /// cannot measure, and then nothing is tried.
    pub(super) fn start(tell: fn(&Said) -> !) -> Result<Meter, String> {
        let unmeasured = || "its trial could not measure its own memory".to_string();
        let mut held = Held::new();
        let before = held.now().ok_or_else(unmeasured)?;
        let meter =
            Meter { wall: Instant::now(), processor: processor(), before, peak: Arc::new(AtomicU64::new(0)), held };
        let mut watching = meter.again();
        let watch = move || {
            loop {
                thread::sleep(EVERY);
                if let Err(over) = watching.cost() {
                    tell(&over);
                }
            }
        };
        thread::Builder::new().spawn(watch).map_err(|_| "its trial could not start its meter".to_string())?;
        Ok(meter)
    }

    /// The same meter, for another thread.
    fn again(&self) -> Meter {
        Meter { peak: Arc::clone(&self.peak), held: Held::new(), ..*self }
    }

    /// What the picture has cost so far, in milliseconds of processor time and KiB of memory, or what is said of
    /// a picture that is over either.
    pub(super) fn cost(&mut self) -> Result<(u64, u64), Said> {
        let unmeasured = || Said::Unsure("its trial could not measure its own memory".to_string());
        let now = self.held.now().ok_or_else(unmeasured)?.saturating_sub(self.before);
        let peak = self.peak.fetch_max(now, Ordering::Relaxed).max(now);
        if peak > MOST_MEMORY {
            let most = MOST_MEMORY / (1024 * 1024);
            return Err(Said::Is(Err(format!("drawing it took more than {most} MiB of memory"))));
        }
        let (used, exact) = match (self.processor, processor()) {
            (Some(before), Some(now)) => (now.saturating_sub(before), true),
            _ => (self.wall.elapsed(), false),
        };
        let most = MOST_PROCESSOR.as_millis();
        if used > MOST_PROCESSOR && exact {
            return Err(Said::Is(Err(format!("drawing it took more than {most} ms of processor time"))));
        }
        if used > MOST_PROCESSOR {
            let why = format!("its trial cannot measure processor time here, and drawing it took more than {most} ms");
            return Err(Said::Unsure(why));
        }
        Ok((u64::try_from(used.as_millis()).unwrap_or(u64::MAX), peak / 1024))
    }
}

#[cfg(test)]
mod tests {
    use super::{MOST_MEMORY, Meter};
    use crate::theme::trial::Said;

    /// What the meter's own thread does in a test when a cost is passed: waits, where the child's would end it.
    fn wait(_: &Said) -> ! {
        loop {
            std::thread::park();
        }
    }

    fn refusal(meter: &mut Meter) -> Option<String> {
        match meter.cost() {
            Err(Said::Is(Err(why))) => Some(why),
            _ => None,
        }
    }

    // Each cost is refused by itself: what is over one here is under the other. One test, since both are costs
    // of the one process the tests run in: time first, while it holds little.
    #[test]
    fn each_cost_over_its_most_is_refused_by_name_and_is_an_answer_about_the_picture() {
        #[cfg(unix)]
        {
            let (mut meter, mut sum) = (Meter::start(wait).unwrap(), 0u64);
            let said = loop {
                (0..200_000u64).for_each(|turn| sum = std::hint::black_box(sum.wrapping_mul(31).wrapping_add(turn)));
                if let Err(said) = meter.cost() {
                    break said;
                }
            };
            let most = super::MOST_PROCESSOR.as_millis();
            assert_eq!(said, Said::Is(Err(format!("drawing it took more than {most} ms of processor time"))));
        }
        let mut meter = Meter::start(wait).unwrap();
        assert!(meter.cost().is_ok(), "nothing was taken yet");
        // Every page of it is written, so the process holds it.
        let held = vec![7u8; MOST_MEMORY as usize + 16 * 1024 * 1024];
        assert_eq!(refusal(&mut meter).as_deref(), Some("drawing it took more than 32 MiB of memory"));
        // A peak stays one after what took it is given back.
        drop(std::hint::black_box(held));
        assert!(refusal(&mut meter).is_some_and(|why| why.ends_with("of memory")));
    }
}
