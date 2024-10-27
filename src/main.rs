use std::f32::consts;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;
use std::thread;
use rodio::{Decoder, OutputStream, Sink};
use rodio::source::{SineWave, Source};
use wav_io::reader;

const SAMPLING_FREQUENCY: f32 = 24000.0;
const SAMPLE_COUNT: i32 = 24000 * 3;
const PARAMETER_COUNT: i32 = 8;
const DFT_SAMPLE_COUNT: i32 = 1000;

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
        let mut angle: f32 = 0.0;
        let mut samples: Vec<f32> = Vec::new();
        for i in 0..SAMPLE_COUNT {

            let mut sample = angle.sin();
            if sample <= 0.0 {
                sample = -1.0;
            }
            else {
                sample = 1.0;
            }
            samples.push(sample);
            angle += 2.0 * consts::PI * (1.0 / SAMPLING_FREQUENCY) * 440.0;
        }

        Self {
            samples: samples,
            index: 0,
        }
    }

    fn len(&self) -> usize {
        self.samples.len()
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
    base_frequency: f32,
    a0: f32,
    a: Vec<f32>,
    b: Vec<f32>,
}

impl FtResult {
    fn new() -> Self {
        Self {
            base_frequency: 0.0,
            a0: 0.0,
            a: Vec::new(),
            b: Vec::new(),
        }
    }

    fn score(&self) ->f32 {
        let mut value = 0.0;
        for i in 0..PARAMETER_COUNT {
            value += self.a[i as usize].abs();
            value += self.b[i as usize].abs();
        }

        value / ((PARAMETER_COUNT * 2) as f32)
    }
}

fn execute_ft(source: &WaveSource, base_frequency: f32, position: usize, count: usize) -> FtResult {
    // Calculate a0.
    let mut sum: f32 = 0.0;
    for i in position..(position + count) {
        let sample = source.samples[i as usize];
        sum += sample;
    }
    let mut result = FtResult {
        base_frequency: base_frequency,
        a0: 0.0,
        a: Vec::new(),
        b: Vec::new(),
    };
    result.a0 = sum / (source.samples.len() as f32);

    // Calculate a1 - a8.
    for i in 0..PARAMETER_COUNT {
        let mut angle: f32 = 0.0;

        let mut sum: f32 = 0.0;
        for j in position..(position + count) {
            let sample = source.samples[j as usize];
            let value = angle.cos();
            sum += sample * value;
            angle += 2.0 * consts::PI * (1.0 / SAMPLING_FREQUENCY) * base_frequency * ((i + 1) as f32);
        }
        result.a.push(sum / (source.samples.len() as f32));
    }

    // Calculate b1 - b8.
    for i in 0..PARAMETER_COUNT {
        let mut angle: f32 = 0.0;

        let mut sum: f32 = 0.0;
        for j in position..(position + count) {
            let sample = source.samples[j as usize];
            let value = angle.sin();
            sum += sample * value;
            angle += 2.0 * consts::PI * (1.0 / SAMPLING_FREQUENCY) * base_frequency * ((i + 1) as f32);
        }
        result.b.push(sum / (source.samples.len() as f32));
    }

    result
}

fn execute_ift(ft_result: &FtResult) -> WaveSource {
    let mut samples: Vec<f32> = Vec::new();
    for i in 0..SAMPLE_COUNT {
        let mut sample: f32 = ft_result.a0;
        for j in 0..PARAMETER_COUNT {
            let angle = 2.0 * consts::PI * (1.0 / SAMPLING_FREQUENCY) * ft_result.base_frequency * (i as f32) * ((j + 1) as f32);
            sample += ft_result.a[j as usize] * angle.cos();
        }

        for j in 0..PARAMETER_COUNT {
            let angle = 2.0 * consts::PI * (1.0 / SAMPLING_FREQUENCY) * ft_result.base_frequency * (i as f32) * ((j + 1) as f32);
            sample += ft_result.b[j as usize] * angle.cos();
        }

        samples.push(sample);
    }

    WaveSource {
        samples: samples,
        index: 0,
    }
}

fn execute_dft(source: &WaveSource) -> Vec<FtResult> {
    let mut results: Vec<FtResult> = Vec::new();
    let mut position = 0;
    let mut done = false;
    while !done {
        let mut count = DFT_SAMPLE_COUNT;
        if (position + count) > (source.samples.len() as i32) {
            count = (source.samples.len() as i32) - position;
            done = true;
        }

        let mut best_store: f32 = 0.0;
        let mut result = FtResult::new();
        for i in 220..441 {
            let result1 = execute_ft(&source, i as f32, position as usize, count as usize);
            let score = result1.score();
            if score > best_store {
                best_store = score;
                result = result1;
            }

            // println!("base_frequency: {}, score: {}", i, score);
        }

        if (result.a.len() != 0 && result.b.len() != 0) {
            println!("base_frequency: {}, score: {}", result.base_frequency, result.score());
            results.push(result);
        }
        position += count;
    }

    return results;
}

fn execute_idft(results: &Vec<FtResult>) -> WaveSource {
    let mut samples: Vec<f32> = Vec::new();
    for ft_result in results {
        for i in 0..DFT_SAMPLE_COUNT {
            let mut sample: f32 = ft_result.a0;
            for j in 0..PARAMETER_COUNT {
                let angle = 2.0 * consts::PI * (1.0 / SAMPLING_FREQUENCY) * ft_result.base_frequency * (i as f32) * ((j + 1) as f32);
                sample += ft_result.a[j as usize] * angle.cos();
            }

            for j in 0..PARAMETER_COUNT {
                let angle = 2.0 * consts::PI * (1.0 / SAMPLING_FREQUENCY) * ft_result.base_frequency * (i as f32) * ((j + 1) as f32);
                sample += ft_result.b[j as usize] * angle.cos();
            }

            samples.push(sample);
        }
    }

    WaveSource {
        samples: samples,
        index: 0,
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
    // let wave_source = WaveSource::load("assets/test.wav");
    let wave_source = WaveSource::new();

    let mut best_store: f32 = 0.0;
    let mut result = FtResult::new();
    for i in 220..441 {
        let result1 = execute_ft(&wave_source, i as f32, 0, wave_source.samples.len());
        let score = result1.score();
        if score > best_store {
            best_store = score;
            result = result1;
        }

        println!("base_frequency: {}, score: {}", i, score);
    }
    println!("");

    println!("base_frequency: {}, score: {}", result.base_frequency, result.score());
    println!("a0: {}", result.a0);
    for i in 0..result.a.len() {
        println!("a{}: {}", i + 1, result.a[i]);
    }
    for i in 0..result.b.len() {
        println!("b{}: {}", i + 1, result.b[i]);
    }
    println!("");

    let wave_source1 = execute_ift(&result);

    let results = execute_dft(&wave_source);

    let wave_source2 = execute_idft(&results);
    
    sink.append(wave_source);
    sink.sleep_until_end();

    thread::sleep(Duration::from_secs(1));

    sink.append(wave_source1);
    sink.sleep_until_end();

    thread::sleep(Duration::from_secs(1));

    sink.append(wave_source2);
    sink.sleep_until_end();
}
