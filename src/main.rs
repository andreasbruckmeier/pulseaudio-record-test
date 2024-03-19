use psimple::Simple;
use pulse::sample::{Format, Spec};
use pulse::stream::Direction;
use std::fs::File;
use std::io::Write;

// Mono = 1, Stereo = 2
const CHANNELS: u8 = 2;

// Samples per second (8000, 44100, e.g.)
const SAMPLING: u32 = 44100;

// how long to record
const DURATION: u32 = 5;

// Size of buffer for reading from pulse audio
const BUFFER_SIZE: usize = 16;

// Intitial size of data vector in seconds
const DATA_INITIAL: usize = 60;

#[derive(Debug)]
enum Sample {
    Mono(i16),
    Stereo((i16, i16)),
}

fn append_buffer(data: &mut Vec<Sample>, buffer: &[u8], stereo: bool) {
    // big endian 16 bit PCM, so we need to combine two bytes
    if stereo {
        data.extend(buffer.chunks_exact(4).map(|chunk| {
            Sample::Stereo((
                i16::from_be_bytes([chunk[0], chunk[1]]),
                i16::from_be_bytes([chunk[2], chunk[3]]),
            ))
        }));
    } else {
        data.extend(
            buffer
                .chunks_exact(2)
                .map(|chunk| Sample::Mono(i16::from_be_bytes([chunk[0], chunk[1]]))),
        );
    }
}

fn write_wav_file(
    data: &Vec<Sample>,
    sample_rate: u32,
    num_channels: u16,
    output_filename: &str,
) -> std::io::Result<()> {
    let data_length = data.len() * 2 * num_channels as usize;
    let bits_per_sample: u16 = 16;
    let mut file = File::create(output_filename)?;

    // Write WAV header
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_length as u32).to_le_bytes())?; // File size
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // Subchunk1Size
    file.write_all(&1u16.to_le_bytes())?; // AudioFormat: PCM
    file.write_all(&num_channels.to_le_bytes())?; // NumChannels
    file.write_all(&sample_rate.to_le_bytes())?; // SampleRate
    file.write_all(
        &((sample_rate * num_channels as u32 * bits_per_sample as u32 / 8) as u32).to_le_bytes(),
    )?; // ByteRate
    file.write_all(&(num_channels * bits_per_sample / 8).to_le_bytes())?; // BlockAlign
    file.write_all(&bits_per_sample.to_le_bytes())?; // BitsPerSample
    file.write_all(b"data")?;
    file.write_all(&(data_length as u32).to_le_bytes())?; // Subchunk2Size

    // Write audio data
    for sample in data {
        match sample {
            Sample::Mono(sample) => {
                let msb = ((sample >> 8) & 0xFF) as u8;
                let lsb = (sample & 0xFF) as u8;
                file.write_all(&[msb, lsb])?;
            }
            Sample::Stereo(sample) => {
                let msb = ((sample.1 >> 8) & 0xFF) as u8;
                let lsb = (sample.1 & 0xFF) as u8;
                file.write_all(&[msb, lsb])?;
                let msb = ((sample.0 >> 8) & 0xFF) as u8;
                let lsb = (sample.0 & 0xFF) as u8;
                file.write_all(&[msb, lsb])?;
            }
        }
    }

    Ok(())
}

fn main() {
    let spec = Spec {
        format: Format::S16NE, // 16bit PCM big endian
        channels: CHANNELS,
        rate: SAMPLING,
    };
    assert!(spec.is_valid());

    let s = Simple::new(
        None,              // default server
        "MyPulseClient",   // application’s name
        Direction::Record, // record from pa server
        None,              // default device
        "Music",           // stream description
        &spec,             // ample format
        None,              // default channel map
        None,              // default buffering attributes
    )
    .unwrap();

    let mut data: Vec<Sample> = Vec::with_capacity(SAMPLING as usize * DATA_INITIAL);
    let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

    while data.len() < (DURATION * SAMPLING) as usize {
        match s.read(&mut buffer) {
            Ok(()) => {
                append_buffer(&mut data, &buffer, CHANNELS > 1);
            }
            Err(err) => eprintln!("error: {}", err),
        }
    }

    let result = write_wav_file(&data, SAMPLING, CHANNELS as u16, "foobar.wav");
    println!("{:#?}", result);
}
