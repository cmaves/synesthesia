use crate::Error;
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use rustfft::FFT;
use std::time::Duration;

pub trait InactiveAudioSource {
    type ActiveType: ActiveAudioSource;
    fn activate(self) -> Result<Self::ActiveType, Error>;
}

pub trait ActiveAudioSource {
    type InactiveType;
    fn deactivate(self) -> Result<Self::InactiveType, Error>;
    fn cur_time(&self) -> u64;
    fn recv(&mut self) -> Result<StereoSample, Error>;
    fn recv_timeout(&mut self, timeout: Duration) -> Result<StereoSample, Error>;
    #[inline]
    fn try_recv(&mut self) -> Result<StereoSample, Error> {
        self.recv_timeout(Duration::from_secs(0))
    }
    fn iter(&mut self) -> Iter<'_, Self>
        where Self: Sized
    {
        Iter { src: self }
    }
    fn try_iter(&mut self) -> TryIter<'_, Self> 
        where Self: Sized
    {
        TryIter { src: self }
    }
}
pub struct Iter<'a, T: ActiveAudioSource> {
    src: &'a mut  T 
}

impl<T: ActiveAudioSource> Iterator for Iter<'_, T>{
    type Item = StereoSample;
    fn next(&mut self) -> Option<Self::Item> {
        self.src.recv().ok()
    }
}

pub struct TryIter<'a, T: ActiveAudioSource> {
    src: &'a mut T 
}
impl<T: ActiveAudioSource> Iterator for TryIter<'_, T>{
    type Item = StereoSample;
    fn next(&mut self) -> Option<Self::Item> {
        self.src.try_recv().ok()
    }
}

#[derive(Debug)]
pub struct StereoSample {
    sample_size: usize,
    left: Vec<f32>,
    right: Vec<f32>,
    rate: u32,
    time: u64,
}

impl StereoSample {
    pub fn new(sample_size: usize, rate: u32, time: u64) -> Self {
        StereoSample {
            sample_size,
            rate,
            left: Vec::with_capacity(sample_size),
            right: Vec::with_capacity(sample_size),
            time,
        }
    }
    pub fn extend(&mut self, left: &[f32], right: &[f32]) -> bool {
        assert_eq!(left.len(), right.len());
        let len = left.len();
        let remaining = self.sample_size - self.left.len();
        if remaining > len {
            self.left.extend_from_slice(left);
            self.right.extend_from_slice(right);
            false //return false if not full
        } else {
            self.left.extend_from_slice(&left[0..remaining]);
            self.right.extend_from_slice(&right[0..remaining]);
            true //return true if full
        }
    }
    pub fn len(&self) -> usize {
        self.left.len()
    }
    pub fn spectrogram<T: FFT<f32>>(&self, fft: &T) -> (Vec<f32>, Vec<f32>) {
        let mut l_in: Vec<Complex<f32>> = self.left.iter().map(|f| Complex::new(*f, 0.0)).collect();
        let mut r_in: Vec<Complex<f32>> =
            self.right.iter().map(|f| Complex::new(*f, 0.0)).collect();
        let mut l_out: Vec<Complex<f32>> = vec![Complex::zero(); self.sample_size];
        let mut r_out: Vec<Complex<f32>> = vec![Complex::zero(); self.sample_size];
        fft.process_multi(&mut l_in, &mut l_out);
        fft.process_multi(&mut r_in, &mut r_out);
        /* normalize complex-valued amp and convert to amp-to-dB log_10 (amp^2).
        Using norm_sqr() is a simplification that
                    allows us to avoid an expensive sqrt operation for a value
                                we would either just sqaure before being input to log10() ( or double after the log10(0).
                                        */
        (
            l_out
                .into_iter()
                .map(|c| c.norm_sqr().log10() * 10.0)
                .collect(),
            r_out
                .into_iter()
                .map(|c| c.norm_sqr().log10() * 10.0)
                .collect(),
        )
    }
}

pub(crate) const WEIGHT: [f32; 256] = [
    0.0, -20.45, -14.43, -10.92, -8.43, -6.50, -4.93, -3.61, -2.47, -1.47, -0.58, 0.22, 0.95, 1.61,
    2.22, 2.79, 3.31, 3.80, 4.26, 4.68, 5.09, 5.47, 5.83, 6.17, 6.49, 6.80, 7.09, 7.37, 7.64, 7.89,
    8.14, 8.37, 8.60, 8.81, 9.02, 9.22, 9.41, 9.59, 9.77, 9.93, 10.10, 10.25, 10.40, 10.54, 10.68,
    10.81, 10.94, 11.06, 11.17, 11.27, 11.38, 11.47, 11.56, 11.64, 11.72, 11.79, 11.85, 11.91,
    11.97, 12.01, 12.05, 12.09, 12.12, 12.14, 12.16, 12.17, 12.18, 12.18, 12.17, 12.16, 12.15,
    12.13, 12.10, 12.07, 12.04, 12.00, 11.95, 11.91, 11.85, 11.79, 11.73, 11.67, 11.60, 11.52,
    11.44, 11.36, 11.27, 11.18, 11.08, 10.98, 10.87, 10.76, 10.64, 10.51, 10.38, 10.24, 10.10,
    9.95, 9.79, 9.63, 9.45, 9.27, 9.08, 8.89, 8.68, 8.47, 8.25, 8.02, 7.78, 7.54, 7.28, 7.02, 6.75,
    6.48, 6.20, 5.91, 5.61, 5.31, 5.01, 4.70, 4.38, 4.06, 3.74, 3.41, 3.09, 2.76, 2.42, 2.09, 1.75,
    1.41, 1.08, 0.74, 0.40, 0.06, -0.28, -0.62, -0.96, -1.29, -1.63, -1.97, -2.30, -2.63, -2.97,
    -3.30, -3.63, -3.95, -4.28, -4.60, -4.93, -5.25, -5.57, -5.88, -6.20, -6.51, -6.83, -7.13,
    -7.44, -7.75, -8.05, -8.35, -8.65, -8.95, -9.25, -9.54, -9.84, -10.13, -10.42, -10.70, -10.99,
    -11.27, -11.55, -11.83, -12.11, -12.38, -12.66, -12.93, -13.20, -13.47, -13.74, -14.00, -14.27,
    -14.53, -14.79, -15.05, -15.31, -15.56, -15.82, -16.07, -16.32, -16.57, -16.82, -17.06, -17.31,
    -17.55, -17.79, -18.04, -18.27, -18.51, -18.75, -18.98, -19.22, -19.45, -19.68, -19.91, -20.14,
    -20.37, -20.59, -20.82, -21.04, -21.27, -21.49, -21.71, -21.93, -22.14, -22.36, -22.58, -22.79,
    -23.00, -23.21, -23.43, -23.64, -23.84, -24.05, -24.26, -24.46, -24.67, -24.87, -25.08, -25.28,
    -25.48, -25.68, -25.88, -26.07, -26.27, -26.47, -26.66, -26.86, -27.05, -27.24, -27.43, -27.62,
    -27.81, -28.00, -28.19, -28.38, -28.56, -28.75, -28.93, -29.12, -29.30, -29.48, -29.66, -29.84,
    -30.02, -30.20, -30.38,
];
