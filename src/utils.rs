use textwrap;

pub fn generate_think_lines(thinking: &str) -> String {
    let width = textwrap::termwidth() - 5;
    "   | ".to_string() + &textwrap::wrap(thinking, width).join("\n   | ")
}