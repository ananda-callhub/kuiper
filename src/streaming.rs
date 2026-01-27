use std::io::{self, Write};
use std::time::Duration;
use tokio::time::sleep;

/// Stream output tokens with simulated delay for visual effect
pub async fn stream_output<I>(tokens: I)
where
    I: IntoIterator<Item = String>,
{
    for token in tokens {
        print!("{}", token);
        io::stdout().flush().ok();
        sleep(Duration::from_millis(50)).await;
    }
    println!();
}

/// Stream output with configurable delay
pub async fn stream_output_with_delay<I>(tokens: I, delay_ms: u64)
where
    I: IntoIterator<Item = String>,
{
    for token in tokens {
        print!("{}", token);
        io::stdout().flush().ok();
        sleep(Duration::from_millis(delay_ms)).await;
    }
    println!();
}

/// Print output with a simulated streaming effect (sync version)
pub fn print_streamed(output: &str) {
    for word in output.split_whitespace() {
        print!("{} ", word);
        io::stdout().flush().ok();
        std::thread::sleep(Duration::from_millis(30));
    }
    println!();
}

/// Stream tokens to stdout as they arrive
pub fn stream_token(token: &str) {
    print!("{}", token);
    io::stdout().flush().ok();
}

/// Print a newline after streaming completes
pub fn stream_end() {
    println!();
}

/// Convert a string response into token iterator (word-based simulation)
pub fn tokenize(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split_whitespace().map(|word| format!("{} ", word))
}

/// Convert a string response into token iterator (char-based for more granular streaming)
pub fn tokenize_chars(text: &str) -> impl Iterator<Item = String> + '_ {
    text.chars().map(|c| c.to_string())
}

/// Format output for terminal display
pub fn format_output(output: &str, max_width: Option<usize>) -> String {
    match max_width {
        Some(width) => textwrap(output, width),
        None => output.to_string(),
    }
}

/// Simple text wrapping
fn textwrap(text: &str, width: usize) -> String {
    let mut result = String::new();
    let mut current_line_len = 0;

    for word in text.split_whitespace() {
        if current_line_len + word.len() + 1 > width && current_line_len > 0 {
            result.push('\n');
            current_line_len = 0;
        } else if current_line_len > 0 {
            result.push(' ');
            current_line_len += 1;
        }
        result.push_str(word);
        current_line_len += word.len();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_textwrap() {
        let input = "This is a longer sentence that should wrap";
        let wrapped = textwrap(input, 20);
        assert!(wrapped.contains('\n'));
    }

    #[test]
    fn test_tokenize() {
        let tokens: Vec<String> = tokenize("hello world").collect();
        assert_eq!(tokens, vec!["hello ", "world "]);
    }

    #[test]
    fn test_tokenize_chars() {
        let tokens: Vec<String> = tokenize_chars("hi").collect();
        assert_eq!(tokens, vec!["h", "i"]);
    }
}
