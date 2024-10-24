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
    sink.append(wave_source);

    // The sound plays in a separate thread. This call will block the current thread until the sink
    // has finished playing all its queued sounds.
    sink.sleep_until_end();
}
