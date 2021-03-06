use std::default;
use std::process::Command;
use std::path::PathBuf;
use std::vec::Vec;

use errors::*;
use output::{OutputAssertion, OutputKind};

/// Assertions for a specific command.
#[derive(Debug)]
pub struct Assert {
    cmd: Vec<String>,
    current_dir: Option<PathBuf>,
    expect_success: Option<bool>,
    expect_exit_code: Option<i32>,
    expect_output: Vec<OutputAssertion>,
}

impl default::Default for Assert {
    /// Construct an assert using `cargo run --` as command.
    ///
    /// Defaults to asserting _successful_ execution.
    fn default() -> Self {
        Assert {
            cmd: vec!["cargo", "run", "--"]
                .into_iter().map(String::from).collect(),
            current_dir: None,
            expect_success: Some(true),
            expect_exit_code: None,
            expect_output: vec![],
        }
    }
}

impl Assert {
    /// Run the crate's main binary.
    ///
    /// Defaults to asserting _successful_ execution.
    pub fn main_binary() -> Self {
        Assert::default()
    }

    /// Run a specific binary of the current crate.
    ///
    /// Defaults to asserting _successful_ execution.
    pub fn cargo_binary(name: &str) -> Self {
        Assert {
            cmd: vec!["cargo", "run", "--bin", name, "--"]
                .into_iter().map(String::from).collect(),
            ..Self::default()
        }
    }

    /// Run a custom command.
    ///
    /// Defaults to asserting _successful_ execution.
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["echo", "1337"])
    ///     .unwrap();
    /// ```
    pub fn command(cmd: &[&str]) -> Self {
        Assert {
            cmd: cmd.into_iter().cloned().map(String::from).collect(),
            ..Self::default()
        }
    }

    /// Add arguments to the command.
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["echo"])
    ///     .with_args(&["42"])
    ///     .stdout().contains("42")
    ///     .unwrap();
    /// ```
    pub fn with_args(mut self, args: &[&str]) -> Self {
        self.cmd.extend(args.into_iter().cloned().map(String::from));
        self
    }

    /// Sets the working directory for the command.
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["wc", "lib.rs"])
    ///     .current_dir(std::path::Path::new("src"))
    ///     .stdout().contains("lib.rs")
    ///     .execute()
    ///     .unwrap();
    /// ```
    pub fn current_dir<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.current_dir = Some(dir.into());
        self
    }

    /// Small helper to make chains more readable.
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["echo", "42"])
    ///     .stdout().contains("42")
    ///     .unwrap();
    /// ```
    pub fn and(self) -> Self {
        self
    }

    /// Expect the command to be executed successfully.
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["echo", "42"])
    ///     .unwrap();
    /// ```
    pub fn succeeds(mut self) -> Self {
        self.expect_exit_code = None;
        self.expect_success = Some(true);
        self
    }

    /// Expect the command to fail.
    ///
    /// Note: This does not include shell failures like `command not found`. I.e. the
    ///       command must _run_ and fail for this assertion to pass.
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["cat", "non-existing-file"])
    ///     .fails()
    ///     .and()
    ///     .stderr().contains("non-existing-file")
    ///     .unwrap();
    /// ```
    pub fn fails(mut self) -> Self {
        self.expect_success = Some(false);
        self
    }

    /// Expect the command to fail and return a specific error code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["cat", "non-existing-file"])
    ///     .fails_with(1)
    ///     .and()
    ///     .stderr().is("cat: non-existing-file: No such file or directory")
    ///     .unwrap();
    /// ```
    pub fn fails_with(mut self, expect_exit_code: i32) -> Self {
        self.expect_success = Some(false);
        self.expect_exit_code = Some(expect_exit_code);
        self
    }

    /// Create an assertion for stdout's contents
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["echo", "42"])
    ///     .stdout().contains("42")
    ///     .unwrap();
    /// ```
    pub fn stdout(self) -> OutputAssertionBuilder {
        OutputAssertionBuilder {
            assertion: self,
            kind: OutputKind::StdOut,
            expected_result: true,
        }
    }

    /// Create an assertion for stdout's contents
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["cat", "non-existing-file"])
    ///     .fails_with(1)
    ///     .and()
    ///     .stderr().is("cat: non-existing-file: No such file or directory")
    ///     .unwrap();
    /// ```
    pub fn stderr(self) -> OutputAssertionBuilder {
        OutputAssertionBuilder {
            assertion: self,
            kind: OutputKind::StdErr,
            expected_result: true,
        }
    }

    /// Execute the command and check the assertions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// let test = assert_cli::Assert::command(&["echo", "42"])
    ///     .stdout().contains("42")
    ///     .execute();
    /// assert!(test.is_ok());
    /// ```
    pub fn execute(self) -> Result<()> {
        let cmd = &self.cmd[0];
        let args: Vec<_> = self.cmd.iter().skip(1).collect();
        let mut command = Command::new(cmd);
        let command = command.args(&args);
        let command = match self.current_dir {
            Some(ref dir) => command.current_dir(dir),
            None => command,
        };
        let output = command.output()?;

        if let Some(expect_success) = self.expect_success {
            if expect_success != output.status.success() {
                let out = String::from_utf8_lossy(&output.stdout).to_string();
                let err = String::from_utf8_lossy(&output.stderr).to_string();
                bail!(ErrorKind::StatusMismatch(
                    self.cmd.clone(),
                    expect_success,
                    out,
                    err,
                ));
            }
        }

        if self.expect_exit_code.is_some() &&
            self.expect_exit_code != output.status.code() {
            let out = String::from_utf8_lossy(&output.stdout).to_string();
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            bail!(ErrorKind::ExitCodeMismatch(
                self.cmd.clone(),
                self.expect_exit_code,
                output.status.code(),
                out,
                err,
            ));
        }

        self.expect_output
            .iter()
            .map(|a| a.execute(&output, &self.cmd))
            .collect::<Result<Vec<()>>>()?;

        Ok(())
    }

    /// Execute the command, check the assertions, and panic when they fail.
    ///
    /// # Examples
    ///
    /// ```rust,should_panic="Assert CLI failure"
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["echo", "42"])
    ///     .fails()
    ///     .unwrap(); // panics
    /// ```
    pub fn unwrap(self) {
        if let Err(err) = self.execute() {
            panic!("{}", err);
        }
    }
}

/// Assertions for command output.
#[derive(Debug)]
pub struct OutputAssertionBuilder {
    assertion: Assert,
    kind: OutputKind,
    expected_result: bool,
}

impl OutputAssertionBuilder {
    /// Negate the assertion predicate
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["echo", "42"])
    ///     .stdout().not().contains("73")
    ///     .unwrap();
    /// ```
    pub fn not(mut self) -> Self {
        self.expected_result = ! self.expected_result;
        self
    }

    /// Expect the command's output to **contain** `output`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["echo", "42"])
    ///     .stdout().contains("42")
    ///     .unwrap();
    /// ```
    pub fn contains<O: Into<String>>(mut self, output: O) -> Assert {
        self.assertion.expect_output.push(OutputAssertion {
            expect: output.into(),
            fuzzy: true,
            expected_result: self.expected_result,
            kind: self.kind,
        });
        self.assertion
    }

    /// Expect the command to output **exactly** this `output`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["echo", "42"])
    ///     .stdout().is("42")
    ///     .unwrap();
    /// ```
    pub fn is<O: Into<String>>(mut self, output: O) -> Assert {
        self.assertion.expect_output.push(OutputAssertion {
            expect: output.into(),
            fuzzy: false,
            expected_result: self.expected_result,
            kind: self.kind,
        });
        self.assertion
    }

    /// Expect the command's output to not **contain** `output`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["echo", "42"])
    ///     .stdout().doesnt_contain("73")
    ///     .unwrap();
    /// ```
    pub fn doesnt_contain<O: Into<String>>(self, output: O) -> Assert {
        self.not().contains(output)
    }

    /// Expect the command to output to not be **exactly** this `output`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate assert_cli;
    ///
    /// assert_cli::Assert::command(&["echo", "42"])
    ///     .stdout().isnt("73")
    ///     .unwrap();
    /// ```
    pub fn isnt<O: Into<String>>(self, output: O) -> Assert {
        self.not().is(output)
    }
}
