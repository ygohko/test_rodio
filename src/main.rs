use std::f32::consts;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;
use rodio::{Decoder, OutputStream, Sink};
use rodio::source::{SineWave, Source};
use wav_io::reader;

struct TestSource {
    count: i32,
}

impl Source for TestSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        44100
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Iterator for TestSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let value: f32;
        if ((self.count / 50) & 1) == 1 {
            value = 1.0;
        }
        else {
            value = 0.0;
        }
        self.count += 1;

        Some(value)
    }
}

impl TestSource {
    fn new() -> Self {
        return Self {
            count: 0,
        }
    }
}

struct WaveSource {
    samples: Vec<f32>,
    index: usize,
}

impl Source for WaveSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        24000
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Iterator for WaveSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let value: f32;
        if self.index >= self.samples.len() {
            return None;
        }

        value = self.samples[self.index];
        self.index += 1;

        Some(value)
    }
}

impl WaveSource {
    fn new() -> Self {
        Self {
            samples: Vec::new(),
            index: 0,
        }
    }

    fn load(path: impl AsRef<Path>) -> Self {
        let mut result = Self {
            samples: Vec::new(),
            index: 0,
        };
        let file = File::open(path).unwrap();
        let (header, samples) = wav_io::read_from_file(file).unwrap();
        println!("header: {:?}", header);
        for sample in samples {
          result.samples.push(sample)  
        }

        result
    }
}

struct FtResult {
    a0: f32,
    a: Vec<f32>,
    b: Vec<f32>,
}

fn execute_ft(source: &WaveSource) -> FtResult {
    // Calculate a0.
    let mut sum: f32 = 0.0;
    for sample in &source.samples {
        sum += sample;
    }
    let mut result = FtResult {
        a0: 0.0,
        a: Vec::new(),
        b: Vec::new(),
    };
    result.a0 = sum / (source.samples.len() as f32);

    const PARAMETER_COUNT: i32 = 8;
    const SAMPLING_FREQUENCY: f32 = 24000.0;
    const BASE_FREQUENCY: f32 = 440.0;
    // Calculate a1 - a8.
    for i in 1..PARAMETER_COUNT {
        let mut angle: f32 = 0.0;

        let mut sum: f32 = 0.0;
        for sample in &source.samples {
            let value = angle.cos();
            sum += sample * value;
            angle += 2.0 * consts::PI * (1.0 / SAMPLING_FREQUENCY) * BASE_FREQUENCY * (i as f32);
        }
        result.a.push(sum / (source.samples.len() as f32));
    }

    // Calculate b1 - b8.
    for i in 1..PARAMETER_COUNT {
        let mut angle: f32 = 0.0;

        let mut sum: f32 = 0.0;
        for sample in &source.samples {
            let value = angle.sin();
            sum += sample * value;
            angle += 2.0 * consts::PI * (1.0 / SAMPLING_FREQUENCY) * BASE_FREQUENCY * (i as f32);
        }
        result.b.push(sum / (source.samples.len() as f32));
    }

    result
}

fn main() {
    println!("Hello, world!");

    // _stream must live as long as the sink
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    /*
    // Add a dummy source of the sake of the example.
    let mut source = SineWave::new(440.0).take_duration(Duration::from_secs_f32(3.0)).amplify(1.0);
    for i in 0..500 {
        println!("next: {}", source.next().unwrap());
    }
    */

    let test_source = TestSource::new();
    let wave_source = WaveSource::load("assets/test.wav");
    let result = execute_ft(&wave_source);

    println!("a0: {}", result.a0);
    for i in 0..result.a.len() {
        println!("a{}: {}", i + 1, result.a[i]);
    }
    for i in 0..result.b.len() {
        println!("b{}: {}", i + 1, result.b[i]);
    }
    
    // TODO: Show results.
    /*
    sink.append(wave_source);

    // The sound plays in a separate thread. This call will block the current thread until the sink
    // has finished playing all its queued sounds.
    sink.sleep_until_end();
    */
}
