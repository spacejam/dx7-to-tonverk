use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand};
use dx7::PatchBank;

mod wav;

mod toml;

fn parse_duration(s: &str) -> Result<Duration, std::num::ParseIntError> {
    let ms: u64 = s.parse()?;
    Ok(Duration::from_millis(ms))
}

/// Generate Elektron Tonverk multisamples from DX7 SYSEX patches
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List all patches in a sysex bank file
    List {
        /// Path to the DX7 sysex bank file
        sysex_file: PathBuf,
    },
    /// Generate multisample from a patch
    Generate {
        /// Path to the DX7 sysex bank file
        sysex_file: PathBuf,

        /// Patch number (0-indexed)
        patch_number: usize,

        /// Key on duration in milliseconds
        #[arg(long, default_value = "2000", value_parser = parse_duration)]
        key_on_duration: Duration,

        /// Minimum MIDI note
        #[arg(long, default_value_t = 60)]
        min_midi_note: u8,

        /// Maximum MIDI note
        #[arg(long, default_value_t = 108)]
        max_midi_note: u8,

        /// Note increment
        #[arg(long, default_value_t = 3)]
        note_increment: u8,
    },
}

fn main() {
    const SAMPLE_RATE: u32 = 44100;

    let args = Args::parse();

    match args.command {
        Commands::List { sysex_file } => {
            let patch_bank_bytes = std::fs::read(&sysex_file).unwrap_or_else(|e| {
                eprintln!("Error reading sysex file '{}': {}", sysex_file.display(), e);
                std::process::exit(1);
            });

            let patch_bank = PatchBank::new(&patch_bank_bytes);

            for (i, patch) in patch_bank.patches.iter().enumerate() {
                println!("{}: {}", i, patch.name());
            }
        }
        Commands::Generate {
            sysex_file,
            patch_number,
            key_on_duration,
            min_midi_note,
            max_midi_note,
            note_increment,
        } => {
            if min_midi_note > 127 {
                eprintln!(
                    "Error: min_midi_note must be <= 127 (got {})",
                    min_midi_note
                );
                std::process::exit(1);
            }

            if max_midi_note > 127 {
                eprintln!(
                    "Error: max_midi_note must be <= 127 (got {})",
                    max_midi_note
                );
                std::process::exit(1);
            }

            if min_midi_note > max_midi_note {
                eprintln!(
                    "Error: min_midi_note ({}) must be <= max_midi_note ({})",
                    min_midi_note, max_midi_note
                );
                std::process::exit(1);
            }

            let patch_bank_bytes = std::fs::read(&sysex_file).unwrap_or_else(|e| {
                eprintln!("Error reading sysex file '{}': {}", sysex_file.display(), e);
                std::process::exit(1);
            });

            let patch_bank = PatchBank::new(&patch_bank_bytes);

            if patch_number >= patch_bank.patches.len() {
                eprintln!(
                    "Error: patch_number {} is out of range (bank has {} patches)",
                    patch_number,
                    patch_bank.patches.len()
                );
                std::process::exit(1);
            }

            let patch = patch_bank.patches[patch_number];
            let name = tonverk_sanitize(&patch.name());

            let pitches_iter = (0..)
                .map(move |i| min_midi_note + i * note_increment)
                .take_while(move |&x| x < max_midi_note)
                .chain(std::iter::once(max_midi_note));

            let pitches: Vec<u8> = pitches_iter.collect();

            let (wav_data, pitch_start_end) =
                wav::generate_wav(patch, &pitches, SAMPLE_RATE, key_on_duration);

            let base_path = std::path::PathBuf::from(name.clone());

            std::fs::create_dir(&name).expect("unable to make directory");

            // write WAV
            let wav_file_name = format!("{}.wav", name);
            let wav_path = base_path.join(&wav_file_name);
            let mut wav_file = std::fs::File::create(wav_path).expect("unable to create wav file");
            wav_file.write_all(&wav_data).unwrap();
            wav_file.sync_all().unwrap();

            // write .elmulti TOML
            let toml_file_name = format!("{}.elmulti", name);
            let toml_path = base_path.join(&toml_file_name);
            let toml_data = toml::format_toml(&name, &pitch_start_end);
            let mut toml_file =
                std::fs::File::create(toml_path).expect("unable to create elmulti file");
            toml_file.write_all(toml_data.as_bytes()).unwrap();
            toml_file.sync_all().unwrap();
        }
    }
}

fn tonverk_sanitize(input: &str) -> String {
    let allowed_symbols = [
        '~', '!', '@', '#', '$', '%', '^', '&', '(', ')', '_', '+', '-', '=', ' ',
    ];
    let allowed_letters = [
        'å', 'ß', 'ä', 'ö', 'ü', 'æ', 'ø', 'ç', 'ñ', 'Å', 'ẞ', 'Ä', 'Ö', 'Ü', 'Æ', 'Ø', 'Ç', 'Ñ',
    ];

    input
        .chars()
        .filter(|c| {
            c.is_ascii_alphanumeric() || allowed_symbols.contains(c) || allowed_letters.contains(c)
        })
        .collect()
}
