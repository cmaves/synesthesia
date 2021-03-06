use crate::audio::{ActiveAudioSource, AudioSourceOptions, InactiveAudioSource, WEIGHT};
use crate::Error;
use lecp::{Command, LedMsg, Sender};
use rustfft::algorithm::Radix4;
use std::cmp::Ordering;

#[derive(Clone, Copy)]
pub enum Algorithm {
    Linear,
    Quadratic,
}
#[derive(Clone, Copy)]
pub enum Effect {
    Stereo4FlatStack(Algorithm, bool),
}
pub struct AudioVisualizer<T: ActiveAudioSource> {
    active: T,
    pub senders: Vec<Box<dyn Sender>>,
    pub effect: Effect,
    radix: Radix4<f32>,
    pub verbose: u8,
}
impl<T: ActiveAudioSource> AudioVisualizer<T> {
    #[inline]
    pub fn new<I>(inactive: I, effect: Effect, options: AudioSourceOptions) -> Result<Self, Error>
    where
        I: InactiveAudioSource<ActiveType = T>,
    {
        let active = inactive.activate(options)?;
        Ok(AudioVisualizer {
            active,
            effect,
            senders: Vec::new(),
            radix: Radix4::new(256, false),
            verbose: 0,
        })
    }
    pub fn process(&mut self) -> Result<(), Error> {
        let ss = if let Some(ss) = self.active.try_iter().last() {
            ss
        } else {
            self.active.recv()?
        };
        if self.senders.len() == 0 {
            return Ok(());
        }
        let (left, right) = ss.spectrogram(&self.radix);
        let mut msgs;
        match self.effect {
            Effect::Stereo4FlatStack(alg, invert) => {
                msgs = self.process_s4fs(left, right, alg, invert)
            }
        }
        if cfg!(debug_assertions) && self.verbose >= 4 {
            eprintln!("Messages to be send: {:?}", msgs);
        }
        for sender in self.senders.iter_mut() {
            let cur_time = sender.get_time();
            for msg in msgs.iter_mut() {
                msg.time = cur_time;
            }
            sender.send(&msgs)?;
        }
        Ok(())
    }
    #[inline]
    pub fn process_loop(&mut self) -> Error {
        loop {
            if let Err(e) = self.process() {
                return e;
            }
        }
    }

    fn process_s4fs(
        &mut self,
        left: Vec<f32>,
        right: Vec<f32>,
        alg: Algorithm,
        invert: bool,
    ) -> [LedMsg; 9] {
        let n_windows = left.len() / 256;
        let n_win = left.len() / 256;
        // average channels
        let mut l_avg = [0.0; 256];
        let mut r_avg = [0.0; 256];
        for i in 0..256 {
            let mut l_sum = 0.0;
            let mut r_sum = 0.0;
            for n in 0..n_windows {
                l_sum += left[i + n * 256];
                r_sum += right[i + n * 256];
            }
            // average and apply weightins
            l_avg[i] = (l_sum / n_win as f32) + WEIGHT[i];
            r_avg[i] = (r_sum / n_win as f32) + WEIGHT[i];
        }
        let f32_max = |s: &[f32]| {
            *s.iter()
                .max_by(|l, r| l.partial_cmp(r).unwrap_or(Ordering::Equal))
                .unwrap()
        };
        let mut l_bins = [0.0; 4];
        l_bins[0] = f32_max(&l_avg[1..3]); // Subwoofer
        l_bins[1] = f32_max(&l_avg[3..6]); // Woofer
        l_bins[2] = f32_max(&l_avg[6..21]); // Midrange
        l_bins[3] = f32_max(&l_avg[21..256]); // Tweeter

        let mut r_bins = [0.0; 4];
        r_bins[0] = f32_max(&r_avg[1..3]);
        r_bins[1] = f32_max(&r_avg[3..6]);
        r_bins[2] = f32_max(&r_avg[6..21]);
        r_bins[3] = f32_max(&r_avg[21..256]);

        if invert {
            std::mem::swap(&mut l_bins, &mut r_bins);
        }

        let mut left = [LedMsg::default(); 4];
        let mut right = [LedMsg::default(); 4];
        let left_i = left.iter_mut().zip(l_bins.iter());
        let right_i = right.iter_mut().zip(r_bins.iter()).rev();
        let iter = left_i.chain(right_i);
        let mut sum = 0;
        // scale to range of [0, 32] u8 and keep track of total sum
        //eprintln!("l_bin: {:?}\nr_bin: {:?}", l_bins, r_bins);
        match alg {
            Algorithm::Linear => {
                for (r, f) in iter {
                    let val = ((f + 35.0) * 2.0 * 0.31).min(31.0).max(0.0).round() as u8;
                    sum += val + 1;
                    r.cmd = Command::FlatStack(val);
                }
            }
            Algorithm::Quadratic => {
                for (r, f) in iter {
                    let val = ((f + 35.0) / 5.0).max(0.0);
                    let val = (val * val * 0.31).min(31.0).round() as u8;
                    sum += val + 1;
                    r.cmd = Command::FlatStack(val);
                }
            }
        }
        let mut ret = [LedMsg::default(); 9];
        ret[0..4].copy_from_slice(&left);
        ret[5..9].copy_from_slice(&right);
        ret[4].cmd = Command::FlatStack(255 - sum);
        for (i, r) in ret.iter_mut().enumerate() {
            r.element = i as u8;
        }
        for (i, r) in ret[0..4].iter_mut().enumerate() {
            r.color = i as u8 + 1;
        }
        for (i, r) in ret[5..9].iter_mut().rev().enumerate() {
            r.color = i as u8 + 1;
        }
        ret
    }
}
