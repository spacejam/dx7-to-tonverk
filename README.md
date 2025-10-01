# tv7 - generate an Elektron Tonverk-compatible multisample from your favorite DX7 patches

## Installation

1. get rust, probably via [rustup](https://rustup.rs)
2. install tv7 via cargo in the CLI: `cargo install tv7`

## Usage

```sh
# list available patches in your dx7 sysex file
tv7 list <path_to_my_dx7_sysex_bank.syx>

# generate Tonverk-compatible multisample
tv7 generate <path_to_my_dx7_sysex_bank.syx> <patch number>
```

Optional parameters for the `generate` subcommand:
```
Generate multisample from a patch

Usage: tv7 generate [OPTIONS] <SYSEX_FILE> <PATCH_NUMBER>

Arguments:
  <SYSEX_FILE>    Path to the DX7 sysex bank file
  <PATCH_NUMBER>  Patch number (0-indexed)

Options:
      --key-on-duration <KEY_ON_DURATION>  Key on duration in milliseconds [default: 2000]
      --min-midi-note <MIN_MIDI_NOTE>      Minimum MIDI note [default: 60]
      --max-midi-note <MAX_MIDI_NOTE>      Maximum MIDI note [default: 108]
      --note-increment <NOTE_INCREMENT>    Note increment [default: 3]
  -h, --help
```
