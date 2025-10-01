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

The `generate` command will create a subdirectory with the name of the patch you've chosen, and within it there will be a wav file and an elmulti file that may work for your Tonverk if you move it over.

I make no claims that this will work for you, but it has worked for me more than once ;)


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

## Example

Try it out with some of the patches in the syx file in this repo:
```sh
# list sysex contents:
tv7 list ./star1-fast-decay.syx
0: *PPG*Vol.1
1: *PPG*Vol.2
2: *PPG*Vol.3
3: *PPG*Vol.4
4: *PPG*Vol.5
5: *PPG*Vol.6
6: *PPG*Vol.7
7: CHORUS   3
8: SYN.BASS 8
9: SYN.HARMO.
10: *m.Voice 3
11: *Belcanto
12: *Cristals
13: *syn.Vox 1
14: *syn.Vox 2
15: *Synclav.1
16: *Synclav.2
17: *Synclav.3
18: SYN.BRA. 3
19: *Vocoder 2
20: *Vocoder 3
21: *Sequence
22: *space Odd
23: *Metalls 1
24: *Metalls 2
25: *Fairl. 2
26: *Fairl. 3
27: Mooger Low
28: *Floating
29: *Gomono
30: MUTE BASS
31: *syn.Orga

# generate some files:
tv7 generate ./star1-fast-decay.syx 9
tv7 generate ./star1-fast-decay.syx 7
tv7 generate ./star1-fast-decay.syx 25

# move the files over. note that non-Tonverk-friendly characters have been stripped out of the names
cp -r SYNHARMO CHORUS\ \ \ 3 Fairl\ 2 /Volumes/Tonverk/User/Multi-sampled\ Instruments

# eject the Tonverk via cli like a hacker
diskutil eject /Volumes/Tonverk
Disk /Volumes/Tonverk ejected
```

Have fun :)
