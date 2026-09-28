use console::style;

pub fn print_banner(version: &str) {
    super::art::print_main_owl_banner(version);
}

#[allow(dead_code)]
pub fn print_text_banner(version: &str) {
    println!(
        "\x1b[38;2;125;211;252;1m   ______           __            \x1b[0m\n\
         \x1b[38;2;147;197;253;1m  / ____/_  _______/ /_____  _____\x1b[0m\n\
         \x1b[38;2;191;219;254;1m / /   / / / / ___/ __/ __ \\/ ___/\x1b[0m\n\
         \x1b[38;2;224;242;254;1m/ /___/ /_/ (__  ) /_/ /_/ (__  ) \x1b[0m\n\
         \x1b[38;2;240;248;255;1m\\____/\\__,_/____/\\__/\\____/____/  \x1b[0m"
    );
    println!(
        " {} {} {}",
        style("❄ Custos").bold().white(),
        style(format!("v{}", version)).dim(),
        style("— Guardian of Agentic Work").italic().dim()
    );
    println!(
        " {}\n",
        style("Human-governed runtime for specialized agentic workflows").dim()
    );
}
