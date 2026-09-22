use console::style;

pub fn print_banner(version: &str) {
    let logo = r#"
 ██████╗ ██╗   ██╗ ███████╗ ████████╗  ██████╗  ███████╗
██╔════╝ ██║   ██║ ██╔════╝ ╚══██╔══╝ ██╔═══██╗ ██╔════╝
██║      ██║   ██║ ███████╗    ██║    ██║   ██║ ███████╗
██║      ██║   ██║ ╚════██║    ██║    ██║   ██║ ╚════██║
╚██████╗ ╚██████╔╝ ███████║    ██║    ╚██████╔╝ ███████║
 ╚═════╝  ╚═════╝  ╚══════╝    ╚═╝     ╚═════╝  ╚══════╝
"#;

    println!("{}", style(logo).cyan().bold());
    println!(
        " {} {} {}",
        style("Custos").bold().white(),
        style(format!("v{}", version)).dim(),
        style("— Guardian of Work").italic().dim()
    );
    println!(
        " {}\n",
        style("Human-governed runtime for specialized agentic workflows").dim()
    );
}
