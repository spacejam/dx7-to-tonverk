pub fn format_toml(name: &str, pitch_start_ends: &[(u8, usize, usize)]) -> String {
    let mut ret = String::new();
    ret.push_str("# ELEKTRON MULTI-SAMPLE MAPPING FORMAT\n");
    ret.push_str("version = 0\n");
    ret.push_str(&format!("name = '{}'\n", name));

    for (pitch, start, end) in pitch_start_ends {
        let formatted = format!(
            r#"
[[key-zones]]
pitch = {pitch}
key-center = {pitch}.0
velocity-layers = [
  {{ velocity = 0.9960785, strategy = "Forward", sample-slots = [
      {{ sample = "{name}.wav", trim-start = {start}, trim-end = {end} }}
  ]}}
]
        "#
        );

        ret.push_str(&formatted);
    }

    ret
}
