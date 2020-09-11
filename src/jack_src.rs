use crate::audio::{ActiveAudioSource, InactiveAudioSource, StereoSample, AudioSourceOptions};
use crate::Error;
use jack::{AsyncClient, AudioIn, Client, Control, NotificationHandler};
use std::mem;
use std::sync::mpsc;
use std::thread::spawn;
use std::time::{Instant, Duration};

pub struct EventHandler {
	target_dur: Duration,
	stats_start_total: Instant,
	stats_period_start: Instant,
	frames_dropped: usize,
	frames_dropped_total: usize

}
impl EventHandler {
	fn new(stats: u16) -> Self {
		let now = Instant::now();
		Self {
			target_dur: Duration::from_secs(stats as u64),
			stats_period_start: now,
			stats_start_total: now,
			frames_dropped: 0,
			frames_dropped_total: 0
		}
	}
}
impl NotificationHandler for EventHandler {
    #[inline]
    fn xrun(&mut self, client: &Client) -> Control {
		self.frames_dropped += 1;
		self.frames_dropped_total += 1;

		let now = Instant::now();
		let since = now.duration_since(self.stats_period_start);
		if since > self.target_dur {
			let since_secs = since.as_secs_f64();
			let fd_f64  = self.frames_dropped as f64;
			let dps = fd_f64 as f64 / since_secs;
			let loss = fd_f64 / (since_secs * (48000.0 / 256.0)) * 100.0;
			eprintln!("Jack Audio frame stats:\n\tPeriod length: {:.1} secs, drops: {}, {:.1} dps, {:.1}% loss", since_secs, self.frames_dropped, dps, loss);

			let since_secs_total = now.duration_since(self.stats_start_total).as_secs_f64();
			let fd_f64  = self.frames_dropped_total as f64;
			let dps = fd_f64 as f64 / since_secs_total;
			let loss = fd_f64 / (since_secs_total * (48000.0 / 756.0 * 100.0));
			eprintln!("\tTotal drops: {} drops, {:.1} dps, {:.1}% loss\n", self.frames_dropped_total, dps, loss);
			
			self.stats_period_start = now;
			self.frames_dropped = 0;
		}
        Control::Continue
    }
}
struct FrameHandler {
    sample: StereoSample,
    sender: mpsc::SyncSender<StereoSample>,
    left: jack::Port<jack::AudioIn>,
    right: jack::Port<jack::AudioIn>,
    sample_size: usize,
}
impl jack::ProcessHandler for FrameHandler {
    fn process(&mut self, client: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        let in_l_sample = self.left.as_slice(ps);
        let in_r_sample = self.right.as_slice(ps);
        if self.sample.extend(in_l_sample, in_r_sample) {
            /* TODO: evaluate potential performance gains by eliminating allocation,
               that will be dropped every 8 audio frames. This is probably
               expensive and could in theory be replace by either some kind of
               mutex protected circular buffer or perhaps passing allocated Vec
               in a return mpsc-channel. Profiling needs to be done to determine
               where this thread is actually spending time. It could be that this
               is just a neccassaryly expensive component.
            */
            let cur_time = client.frames_to_time(client.frame_time());
            let mut ss = StereoSample::new(self.sample_size, client.sample_rate() as u32, cur_time);
            mem::swap(&mut ss, &mut self.sample); // in theory this should run without issue
            self.sender.send(ss).unwrap();
        }
        jack::Control::Continue
    }
}
impl InactiveAudioSource for Client {
    type ActiveType = JackSource;
    fn activate(self, options: AudioSourceOptions) -> Result<Self::ActiveType, Error> {
        let left = self.register_port("synesthesia_left", AudioIn)?;
        let right = self.register_port("synesthesia_right", AudioIn)?;
        let (sender, recv) = mpsc::sync_channel(1);
        let handler = FrameHandler {
            sample: StereoSample::new(
                768,
                self.sample_rate() as u32,
                self.frames_to_time(self.frame_time()),
            ),
            left,
            right,
            sender,
            sample_size: 768,
        };
		let ev = EventHandler::new(options.stats);
        let a_client = self.activate_async(ev, handler)?;
        Ok(JackSource { a_client, recv })
    }
}
pub struct JackSource {
    a_client: AsyncClient<EventHandler, FrameHandler>,
    recv: mpsc::Receiver<StereoSample>,
}
impl ActiveAudioSource for JackSource {
    type InactiveType = Client;
    #[inline]
    fn deactivate(self) -> Result<Self::InactiveType, Error> {
        Ok(self.a_client.deactivate()?.0)
    }
    #[inline]
    fn cur_time(&self) -> u64 {
        let client = self.a_client.as_client();
        client.frames_to_time(client.frame_time())
    }
    fn recv(&mut self) -> Result<StereoSample, Error> {
        self.recv
            .recv()
            .map_err(|_| Error::Unrecoverable("Audio producer is disconnected".to_string()))
    }
    fn recv_timeout(&mut self, timeout: Duration) -> Result<StereoSample, Error> {
        self.recv.recv_timeout(timeout).map_err(|x| match x {
            mpsc::RecvTimeoutError::Timeout => Error::Timeout("Audio timed out".to_string()),
            mpsc::RecvTimeoutError::Disconnected => {
                Error::Unrecoverable("Audio producer is disconnected".to_string())
            }
        })
    }
}
