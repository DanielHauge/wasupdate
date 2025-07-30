use rhai::EvalAltResult;

pub fn fetch(url: &str) -> Result<String, Box<EvalAltResult>> {
    let Ok(response) = reqwest::blocking::get(url) else {
        let error_msg = format!("Failed to fetch URL: {}", url);
        return Err(error_msg.into());
    };
    if response.status().is_success() {
        let Ok(body) = response.text() else {
            let error_msg = format!("Failed to read response body from URL: {}", url);
            return Err(error_msg.into());
        };
        Ok(body)
    } else {
        let error_msg = format!(
            "Failed to fetch URL: {} with status: {}",
            url,
            response.status()
        );
        Err(error_msg.into())
    }
}

pub fn run(cmd: &str) -> Result<String, Box<EvalAltResult>> {
    let command_parts: Vec<&str> = cmd.split_whitespace().collect();
    let command = command_parts[0];
    let args = &command_parts[1..];
    let output = match std::process::Command::new(command).args(args).output() {
        Ok(o) => o,
        Err(e) => {
            let error_msg = format!("Failed to execute command '{}': {}", cmd, e);
            return Err(error_msg.into());
        }
    };
    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(output_str)
    } else {
        let error_msg = format!("Command '{}' failed with status: {}", cmd, output.status);
        Err(error_msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch() {
        let url = "https://www.google.com";
        let result = fetch(url);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_run() {
        let cmd = "echo Hello, World!";
        let result = run(cmd);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "Hello, World!");
    }
}
