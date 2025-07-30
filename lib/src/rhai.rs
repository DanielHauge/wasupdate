use std::{fmt::format, path::PathBuf};

use rhai::{AST, Engine, EvalAltResult, Scope};
use semver::{Op, Version};

use crate::{install::install_archive, utilities};

pub type RhaiResult<T> = std::result::Result<T, Box<EvalAltResult>>;

pub enum Script {
    File(PathBuf),
    Inline(String),
}

pub struct WasaupEngine {
    engine: Engine,
    ast: AST,
}

const CURRENT_VERSION_FN: &str = "current_version";
const LATEST_VERSION_FN: &str = "latest_version";
const INSTALL_VERSION_FN: &str = "install_version";

impl WasaupEngine {
    pub fn current_version(&self) -> RhaiResult<Version> {
        let semver_str =
            self.engine
                .call_fn::<String>(&mut Scope::new(), &self.ast, CURRENT_VERSION_FN, ())?;
        let semver = match semver::Version::parse(&semver_str) {
            Ok(version) => version,
            Err(e) => {
                let error_msg = format!("Failed to parse '{semver_str}' as current version: {}", e);
                return Err(error_msg.into());
            }
        };

        Ok(semver)
    }

    pub fn latest_version(&self) -> RhaiResult<Version> {
        let semver_str =
            self.engine
                .call_fn::<String>(&mut Scope::new(), &self.ast, LATEST_VERSION_FN, ())?;
        let semver = match semver::Version::parse(&semver_str) {
            Ok(version) => version,
            Err(e) => {
                let error_msg = format!("Failed to parse '{semver_str}' as latest version: {}", e);
                return Err(error_msg.into());
            }
        };

        Ok(semver)
    }

    pub fn install_version(&self, version: &str) -> RhaiResult<String> {
        let archive_loc = self.engine.call_fn::<String>(
            &mut Scope::new(),
            &self.ast,
            INSTALL_VERSION_FN,
            (version.to_string(),),
        )?;
        Ok(archive_loc)
    }

    pub fn new(script: Script) -> RhaiResult<WasaupEngine> {
        let mut engine = Engine::new();
        engine.register_fn("fetch", utilities::fetch);
        engine.register_fn("run", utilities::run);
        let ast = match script {
            Script::File(path) => engine.compile_file(path)?,
            Script::Inline(code) => engine.compile(code.as_str())?,
        };

        let mut has_latest_version = false;
        let mut has_current_version = false;
        let mut has_install_version = false;
        for func in ast.iter_functions() {
            match func.name {
                LATEST_VERSION_FN => {
                    if !func.params.is_empty() {
                        let error_msg = format!(
                            "Function '{LATEST_VERSION_FN}' should not have any parameters, found: {}",
                            func.params.len()
                        );
                        return Err(error_msg.into());
                    }
                    if func.access.is_private() {
                        let error_msg =
                            format!("Function '{LATEST_VERSION_FN}' should not be private");
                        return Err(error_msg.into());
                    }
                    has_latest_version = true
                }
                CURRENT_VERSION_FN => {
                    if !func.params.is_empty() {
                        let error_msg = format!(
                            "Function '{CURRENT_VERSION_FN}' should not have any parameters, found: {}",
                            func.params.len()
                        );
                        return Err(error_msg.into());
                    }
                    if func.access.is_private() {
                        let error_msg =
                            format!("Function '{CURRENT_VERSION_FN}' should not be private");
                        return Err(error_msg.into());
                    }
                    has_current_version = true
                }
                INSTALL_VERSION_FN => {
                    // Check if the function has exactly one parameter
                    if func.params.len() != 1 {
                        let error_msg = format!(
                            "Function '{INSTALL_VERSION_FN}' should have exactly one parameter, found: {}",
                            func.params.len()
                        );
                        return Err(error_msg.into());
                    }
                    // Check if the parameter is a string
                    if func.params[0] != "version" {
                        let error_msg = format!(
                            "Function '{INSTALL_VERSION_FN}' should have a string parameter named 'version'"
                        );
                        return Err(error_msg.into());
                    }
                    // Check if the function is public
                    if func.access.is_private() {
                        let error_msg =
                            format!("Function '{INSTALL_VERSION_FN}' should not be private");
                        return Err(error_msg.into());
                    }
                    has_install_version = true
                }
                _ => {}
            }
        }

        if !has_latest_version {
            return Err(format!("Function '{LATEST_VERSION_FN}' is required but not found").into());
        }
        if !has_current_version {
            return Err(
                format!("Function '{CURRENT_VERSION_FN}' is required but not found").into(),
            );
        }
        if !has_install_version {
            return Err(
                format!("Function '{INSTALL_VERSION_FN}' is required but not found").into(),
            );
        }

        Ok(Self { engine, ast })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_LATEST_VERSION: &str = r#"
        fn latest_version() {
            return "1.0.0";
        }"#;
    const TEST_CURRENT_VERSION: &str = r#"
        fn current_version() {
            return "0.9.0";
        }"#;
    const TEST_INSTALL_VERSION: &str = r#"
        fn install_version(version) {
            return "path/to/archive-" + version + ".tar.gz";
        }"#;

    #[test]
    fn test_new_engine_missing_func_current() {
        let inline_script = format!("{}\n{}", TEST_LATEST_VERSION, TEST_INSTALL_VERSION);
        let script = Script::Inline(inline_script);
        let engine_error = WasaupEngine::new(script).err().unwrap();
        assert_eq!(
            engine_error.to_string(),
            format!("Runtime error: Function '{CURRENT_VERSION_FN}' is required but not found")
        );
    }

    #[test]
    fn test_new_engine_missing_func_latest() {
        let inline_script = format!("{}\n{}", TEST_CURRENT_VERSION, TEST_INSTALL_VERSION);
        let script = Script::Inline(inline_script);
        let engine_error = WasaupEngine::new(script).err().unwrap();
        assert_eq!(
            engine_error.to_string(),
            format!("Runtime error: Function '{LATEST_VERSION_FN}' is required but not found")
        );
    }

    #[test]
    fn test_new_engine_missing_func_install() {
        let inline_script = format!("{}\n{}", TEST_CURRENT_VERSION, TEST_LATEST_VERSION);
        let script = Script::Inline(inline_script);
        let engine_error = WasaupEngine::new(script).err().unwrap();
        assert_eq!(
            engine_error.to_string(),
            format!("Runtime error: Function '{INSTALL_VERSION_FN}' is required but not found")
        );
    }

    #[test]
    fn test_new_engine_valid_script() {
        let inline_script = format!(
            "{}\n{}\n{}",
            TEST_CURRENT_VERSION, TEST_LATEST_VERSION, TEST_INSTALL_VERSION
        );
        let script = Script::Inline(inline_script);
        let engine = WasaupEngine::new(script).expect("Failed to create WasaupEngine");

        // Test current version
        let current_version = engine
            .current_version()
            .expect("Failed to get current version");
        assert_eq!(current_version.to_string(), "0.9.0");

        // Test latest version
        let latest_version = engine
            .latest_version()
            .expect("Failed to get latest version");
        assert_eq!(latest_version.to_string(), "1.0.0");

        // Test install version
        let install_path = engine
            .install_version("1.0.0")
            .expect("Failed to install version");
        assert_eq!(install_path, "path/to/archive-1.0.0.tar.gz");
    }
}
