use jmespath::{Variable, compile};
use rhai::EvalAltResult;
use serde_json::json;

pub fn fetch(url: &str) -> Result<String, Box<EvalAltResult>> {
    let Ok(response) = reqwest::blocking::get(url) else {
        let error_msg = format!("Failed to fetch URL: {url}");
        return Err(error_msg.into());
    };
    if response.status().is_success() {
        let Ok(body) = response.text() else {
            let error_msg = format!("Failed to read response body from URL: {url}");
            return Err(error_msg.into());
        };
        Ok(body)
    } else {
        let response_status = response.status();
        let error_msg = format!("Failed to fetch URL: {url} with status: {response_status}");
        Err(error_msg.into())
    }
}

pub fn jq(json_str: &str, query: &str) -> Result<String, Box<EvalAltResult>> {
    let expr = match compile(query) {
        Ok(k) => k,
        Err(e) => {
            let error_msg = format!("Failed to compile JMESPath query: {query}, error: {e}");
            return Err(error_msg.into());
        }
    };
    let json_var = Variable::from_json(json_str)
        .map_err(|e| format!("Failed to convert JSON string to variable: {e}"))?;
    let result = match expr.search(json_var) {
        Ok(res) => res,
        Err(e) => {
            let error_msg = format!("Failed to execute JMESPath query: {query}, error: {e}");
            return Err(error_msg.into());
        }
    };
    Ok(result.to_string())
}

pub fn run(cmd: &str) -> Result<String, Box<EvalAltResult>> {
    let command_parts: Vec<&str> = cmd.split_whitespace().collect();
    let command = command_parts[0];
    let args = &command_parts[1..];
    let output = match std::process::Command::new(command).args(args).output() {
        Ok(o) => o,
        Err(_) => {
            let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let current_dir = current_exe
                .parent()
                .ok_or("Current executable has no parent directory")?;
            let cmd_path = current_dir.join(command);
            let output = std::process::Command::new(cmd_path).args(args).output();
            match output {
                Ok(o) => o,
                Err(e) => {
                    let error_msg = format!("Failed to execute command '{cmd}': {e}");
                    return Err(error_msg.into());
                }
            }
        }
    };
    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(output_str)
    } else {
        let output_status = output.status;
        let error_msg = format!("Command '{cmd}' failed with status: {output_status}");
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
