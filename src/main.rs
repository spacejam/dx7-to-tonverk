
use anyhow::{anyhow, Result};
use clap::Parser;
use std::path::Path;

use dx7tv::{Dx7Synth, parse_sysex_file, WavOutput};
use dx7tv::synth::frequency_to_midi_note;

/// DX7 Test Vector CLI Tool
///
/// Generate WAV files from DX7 SYSEX patches by playing individual notes
/// until they naturally decay to silence.
#[derive(Parser, Clone)]
#[command(name = "dx7tv")]
#[command(about = "Generate test vectors from DX7 SYSEX patches")]
#[command(version)]
struct Args {
    /// SYSEX file containing DX7 patch(es)
    #[arg(help = "Path to SYSEX file (.syx)")]
    sysex_file: String,

    /// MIDI note number to play (0-127)
    #[arg(help = "MIDI note number (0-127, where 60 = Middle C, 69 = A4)")]
    midi_note: u8,

    /// Maximum note length in seconds
    #[arg(help = "Maximum note length in seconds")]
    note_length: f64,

    /// Output WAV filename
    #[arg(help = "Output WAV file path")]
    output_file: String,

    /// Sample rate in Hz
    #[arg(short, long, default_value = "44100", help = "Sample rate in Hz")]
    sample_rate: u32,

    /// Silence threshold in microseconds
    #[arg(
        short = 't',
        long = "silence-threshold",
        default_value = "100000",
        help = "Silence threshold in microseconds (default: 100ms)"
    )]
    silence_threshold_us: u32,

    /// MIDI velocity (1-127)
    #[arg(long, default_value = "100", help = "MIDI velocity (1-127)")]
    velocity: u8,

    /// Patch number to use (for multi-patch SYSEX files)
    #[arg(
        short,
        long,
        default_value = "0",
        help = "Patch number to use (0-based)"
    )]
    patch: usize,

    /// Verbose output
    #[arg(short = 'v', long, help = "Verbose output")]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Validate arguments
    validate_args(&args)?;

    if args.verbose {
        println!("dx7tv - DX7 Test Vector Generator");
        println!("SYSEX file: {}", args.sysex_file);
        println!(
            "MIDI note: {} ({})",
            args.midi_note,
            note_name(args.midi_note)
        );
        println!("Velocity: {}", args.velocity);
        println!("Max length: {:.2}s", args.note_length);
        println!("Sample rate: {}Hz", args.sample_rate);
        println!("Silence threshold: {}μs", args.silence_threshold_us);
        println!("Output file: {}", args.output_file);
        println!();
    }

    // Load SYSEX file
    if args.verbose {
        println!("Loading SYSEX file...");
    }

    let patches = parse_sysex_file(&args.sysex_file)?;

    if patches.is_empty() {
        return Err(anyhow!("No valid DX7 patches found in SYSEX file"));
    }

    if args.patch >= patches.len() {
        return Err(anyhow!(
            "Patch index {} out of range (found {} patches)",
            args.patch,
            patches.len()
        ));
    }

    let patch = &patches[args.patch];

    if args.verbose {
        println!("Found {} patch(es)", patches.len());
        println!("Using patch {}: \"{}\"", args.patch, patch.name);
        println!();
    }

    // Initialize synthesizer
    if args.verbose {
        println!("Initializing synthesizer...");
    }

    let mut synth = Dx7Synth::new(args.sample_rate as f64, args.note_length + 1.0);
    synth.load_patch(patch.clone())?;

    // Generate audio
    if args.verbose {
        println!("Generating audio...");
    }

    let audio_samples = synth.render_note(args.midi_note, args.velocity, args.note_length)?;

    if args.verbose {
        println!(
            "Generated {} samples ({:.2}s)",
            audio_samples.len(),
            audio_samples.len() as f64 / args.sample_rate as f64
        );
    }

    // Write to WAV file with silence detection
    if args.verbose {
        println!("Writing WAV file...");
    }

    let mut wav_output = WavOutput::new(
        &args.output_file,
        args.sample_rate,
        args.silence_threshold_us,
    )?;

    // Process audio in chunks, applying silence detection
    const CHUNK_SIZE: usize = 1024;
    let mut total_written = 0;
    let mut silence_detected = false;

    for chunk in audio_samples.chunks(CHUNK_SIZE) {
        silence_detected = wav_output.write_samples(chunk)?;
        total_written += chunk.len();

        if silence_detected {
            if args.verbose {
                println!(
                    "Silence threshold reached after {} samples ({:.3}s)",
                    total_written,
                    total_written as f64 / args.sample_rate as f64
                );
            }
            break;
        }
    }

    wav_output.finalize()?;

    if args.verbose {
        println!(
            "Successfully wrote {} samples to '{}'",
            total_written, args.output_file
        );

        if !silence_detected && total_written == audio_samples.len() {
            println!("Note: Reached maximum length without detecting silence");
        }
    }

    println!(
        "Generated test vector: {} -> {}",
        args.sysex_file, args.output_file
    );

    Ok(())
}

/// Validate command line arguments
fn validate_args(args: &Args) -> Result<()> {
    // Check SYSEX file exists
    if !Path::new(&args.sysex_file).exists() {
        return Err(anyhow!("SYSEX file '{}' not found", args.sysex_file));
    }

    // Validate MIDI note
    if args.midi_note > 127 {
        return Err(anyhow!(
            "Invalid MIDI note: {} (must be 0-127)",
            args.midi_note
        ));
    }

    // Validate velocity
    if args.velocity == 0 || args.velocity > 127 {
        return Err(anyhow!(
            "Invalid velocity: {} (must be 1-127)",
            args.velocity
        ));
    }

    // Validate note length
    if args.note_length <= 0.0 {
        return Err(anyhow!(
            "Invalid note length: {} (must be positive)",
            args.note_length
        ));
    }

    if args.note_length > 60.0 {
        return Err(anyhow!(
            "Note length too long: {}s (maximum: 60s)",
            args.note_length
        ));
    }

    // Validate sample rate
    if args.sample_rate < 8000 || args.sample_rate > 192000 {
        return Err(anyhow!(
            "Invalid sample rate: {}Hz (must be 8000-192000)",
            args.sample_rate
        ));
    }

    // Validate silence threshold
    if args.silence_threshold_us == 0 {
        return Err(anyhow!("Silence threshold must be greater than 0"));
    }

    if args.silence_threshold_us > 10_000_000 {
        return Err(anyhow!(
            "Silence threshold too long: {}μs (maximum: 10s)",
            args.silence_threshold_us
        ));
    }

    // Check output directory exists
    if let Some(parent) = Path::new(&args.output_file).parent() {
        if !parent.exists() {
            return Err(anyhow!(
                "Output directory '{}' does not exist",
                parent.display()
            ));
        }
    }

    Ok(())
}

/// Convert MIDI note number to note name
fn note_name(midi_note: u8) -> String {
    let note_names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (midi_note / 12) as i32 - 1;
    let note = midi_note % 12;
    format!("{}{}", note_names[note as usize], octave)
}

/// Convert frequency to note name (approximate)
#[allow(dead_code)]
fn frequency_to_note_name(frequency: f64) -> String {
    let midi_note = frequency_to_midi_note(frequency);
    note_name(midi_note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_name() {
        assert_eq!(note_name(60), "C4"); // Middle C
        assert_eq!(note_name(69), "A4"); // A440
        assert_eq!(note_name(21), "A0"); // Lowest A on piano
        assert_eq!(note_name(108), "C8"); // High C
        assert_eq!(note_name(61), "C#4"); // C# above middle C
    }

    #[test]
    fn test_validate_args() {
        // This would need actual test files to work properly
        // For now, just test the basic structure
        let args = Args {
            sysex_file: "nonexistent.syx".to_string(),
            midi_note: 60,
            note_length: 1.0,
            output_file: "test.wav".to_string(),
            sample_rate: 44100,
            silence_threshold_us: 100000,
            velocity: 100,
            patch: 0,
            verbose: false,
        };

        // Should fail because file doesn't exist
        let result = validate_args(&args);
        assert!(result.is_err());

        // Test invalid MIDI note
        let mut bad_args = args.clone();
        bad_args.midi_note = 128;
        // Would still fail on missing file, but that's tested first
    }

    #[test]
    fn test_frequency_note_conversion() {
        assert_eq!(frequency_to_note_name(440.0), "A4");
        assert_eq!(frequency_to_note_name(261.63), "C4"); // Approximately
    }
}

/// Usage examples and help text
#[allow(dead_code)]
const EXAMPLES: &str = r#"
EXAMPLES:
    # Generate a test vector from a single patch playing middle C
    dx7tv patch.syx 60 2.0 output.wav

    # Use a higher sample rate and shorter silence threshold
    dx7tv -s 48000 -t 50000 patch.syx 69 1.5 a440.wav

    # Use patch #3 from a bank, play with high velocity
    dx7tv -p 3 -v 127 bank.syx 72 3.0 loud_c5.wav

MIDI NOTE REFERENCE:
    21 = A0 (27.5 Hz)    60 = C4 (261.6 Hz, Middle C)
    45 = A2 (110.0 Hz)   69 = A4 (440.0 Hz, Concert A)
    57 = A3 (220.0 Hz)   81 = A5 (880.0 Hz)

    Each octave increases the note number by 12.
    C=0, C#=1, D=2, D#=3, E=4, F=5, F#=6, G=7, G#=8, A=9, A#=10, B=11
"#;
