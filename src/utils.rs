use anyhow::Context;

/// Format a number to a human readable form
/// Mainly used for formattiong model context length
pub fn format_number(n: i64) -> String {
    match n {
        n if n >= 1_000_000_000 => format!("{:.1}B", n as f64 / 1_000_000_000.0),
        n if n >= 1_000_000 => format!("{:.1}M", n as f64 / 1_000_000.0),
        n if n >= 1_000 => format!("{:.1}K", n as f64 / 1_000.0),
        n => n.to_string(),
    }
}

// Convert representations like 3k, 4.5M to i64
pub fn parse_human_number(s: &str) -> anyhow::Result<i64> {
    let s = s.trim();
    let (num_str, multiplier) = match s.chars().last() {
        Some('k') | Some('K') => (&s[..s.len() - 1], 1_000),
        Some('m') | Some('M') => (&s[..s.len() - 1], 1_000_000),
        Some('b') | Some('B') => (&s[..s.len() - 1], 1_000_000_000),
        _ => (s, 1),
    };

    let value: f64 = num_str.parse().context(format!("Invalid number: '{s}'"))?;

    Ok((value * multiplier as f64).round() as i64)
}