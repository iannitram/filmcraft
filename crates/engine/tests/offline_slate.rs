//! The offline slate under a thread pool.
//!
//! A CPU export renders the frames of a batch on the rayon pool. When the sequence shows a file
//! that is missing, several frames of the batch ask the same [`SlateSource`] for its slate, and
//! the first of them draws it. Drawing runs parallel loops of its own, so the thread that draws
//! may wait for another pool thread, and a pool thread that waits takes other work meanwhile.

use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use filmcraft_engine::media_pool::SlateSource;
use filmcraft_media::generators::GeneratorSource;
use filmcraft_media::{FrameRequest, Generator, MediaSource};
use filmcraft_render::offline::{OfflineReason, slate_rgba8};
use filmcraft_time::Tick;

const W: u32 = 1280;
const H: u32 = 720;
const NAME: &str = "missing.mov";
/// Far longer than a round can take (one round draws a slate or two: milliseconds).
const LIMIT: Duration = Duration::from_secs(15);
/// The moments, spread over one drawing and a little beyond, at which the second pool thread
/// comes free.
const STEPS: u32 = 32;
const PASSES: u32 = 3;

/// A second request for a slate that is still being drawn must not stop the pool.
///
/// The pool has two threads. One is kept busy while the other starts drawing the slate for the
/// first request; a second request for the same slate waits in the pool's queue. Then the busy
/// thread comes free, at a moment that moves through the drawing from round to round. When that
/// moment falls inside one of the drawing's parallel loops, the freed thread takes a share of the
/// loop, and the drawing thread, once it has nothing left but to wait for that share, takes the
/// queued request. With the source's lock held across the drawing, that request waited for a lock
/// its own thread held: nothing moved again, which is how an export with missing media stopped.
#[test]
fn a_slate_asked_for_again_while_it_is_drawn_does_not_stop_the_pool() {
    let info = GeneratorSource::new(Generator::ColorMatte { color: [0.0; 4] }, W, H, Default::default(), Tick(100)).info().clone();
    let request = FrameRequest { time: Tick(0), scale: 1.0 };
    let fresh = || Arc::new(SlateSource::new(info.clone(), NAME, OfflineReason::Missing));
    let pool = rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap();

    // What the slate must look like, and how long one drawing takes here. The reference drawing
    // comes first because the first text of a process also loads the fonts.
    let want = slate_rgba8(W as usize, H as usize, NAME, OfflineReason::Missing);
    let timed = fresh();
    let t = Instant::now();
    let first = pool.install(|| timed.video_frame(request).ok()).unwrap();
    let drawing = t.elapsed();
    assert_eq!((first.width, first.height), (W, H));
    assert_eq!(first.to_rgba8(), want, "the slate from the pool is the slate");
    assert_eq!(timed.video_frame(request).unwrap().to_rgba8(), want, "and so is the kept copy");

    for round in 0..PASSES * STEPS {
        // on its own the drawing takes longer than it did with two threads: sweep twice its time
        let delay = drawing.mul_f64(2.0 * f64::from(round % STEPS) / f64::from(STEPS));
        let slate = fresh();
        let (held, is_held) = mpsc::channel::<()>();
        let (release, released) = mpsc::channel::<()>();
        let (done, is_done) = mpsc::channel();
        // one pool thread is kept busy
        pool.spawn(move || {
            let _ = held.send(());
            let _ = released.recv_timeout(LIMIT);
        });
        is_held.recv_timeout(LIMIT).expect("the pool runs a task");
        // two frames that show the same missing file: the free thread draws the slate for the
        // first, the second waits in the pool's queue
        for _ in 0..2 {
            let (slate, done) = (slate.clone(), done.clone());
            pool.spawn(move || {
                let _ = done.send(slate.video_frame(request).ok().map(|f| (f.width, f.height)));
            });
        }
        // part-way through the drawing the busy thread comes free (a spin, not a sleep: the
        // moments are fractions of a millisecond apart)
        let t = Instant::now();
        while t.elapsed() < delay {
            std::hint::spin_loop();
        }
        let _ = release.send(());
        for _ in 0..2 {
            match is_done.recv_timeout(LIMIT) {
                Ok(size) => assert_eq!(size, Some((W, H)), "round {round}"),
                Err(_) => panic!(
                    "round {round}: the pool stopped. A second request for the slate, {delay:?} into a drawing that takes about {drawing:?}, never came back: the drawing thread waits for a lock it holds itself"
                ),
            }
        }
    }
}
