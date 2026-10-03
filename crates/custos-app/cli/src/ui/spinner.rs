use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub struct CliSpinner {
    pb: ProgressBar,
}

impl CliSpinner {
    pub fn new(initial_msg: impl Into<String>) -> Self {
        let pb = ProgressBar::new_spinner();
        let template = "{spinner:.cyan} {msg}";

        if let Ok(style) = ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template(template)
        {
            pb.set_style(style);
        }

        pb.set_message(initial_msg.into());
        pb.enable_steady_tick(Duration::from_millis(80));

        Self { pb }
    }

    pub fn set_message(&self, msg: impl Into<String>) {
        self.pb.set_message(msg.into());
    }

    pub fn finish_success(&self, msg: &str) {
        self.pb.finish_and_clear();
        println!("{} {}", style("✔").cyan().bold(), style(msg).white());
    }

    #[allow(dead_code)]
    pub fn finish_error(&self, msg: &str) {
        self.pb.finish_and_clear();
        println!("{} {}", style("✖").red().bold(), style(msg).red());
    }

    #[allow(dead_code)]
    pub fn finish_and_clear(&self) {
        self.pb.finish_and_clear();
    }
}
