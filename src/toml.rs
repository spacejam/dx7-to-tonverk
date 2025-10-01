pub fn format_toml(name: &str, pitch_start_ends: &[(u8, usize, usize)]) -> String {
    let mut ret = String::new();
    ret.push_str("# ELEKTRON MULTI-SAMPLE MAPPING FORMAT\n");
    ret.push_str("version = 0\n");
    ret.push_str(&format!("name = '{}'\n", name));

    let num_entries = pitch_start_ends.len();
    for (i, (pitch, start, end)) in pitch_start_ends.iter().enumerate() {
        let is_last = i == num_entries - 1;

        let formatted = if is_last {
            format!(
                r#"
[[key-zones]]
pitch = {pitch}
key-center = {pitch}.0

[[key-zones.velocity-layers]]
velocity = 0.9960785
strategy = 'Forward'

[[key-zones.velocity-layers.sample-slots]]
sample = '{name}.wav'
trim-start = {start}
"#
            )
        } else {
            format!(
                r#"
[[key-zones]]
pitch = {pitch}
key-center = {pitch}.0

[[key-zones.velocity-layers]]
velocity = 0.9960785
strategy = 'Forward'

[[key-zones.velocity-layers.sample-slots]]
sample = '{name}.wav'
trim-start = {start}
trim-end = {end}
"#
            )
        };

        ret.push_str(&formatted);
    }

    ret
}
