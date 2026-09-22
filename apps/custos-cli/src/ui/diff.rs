use console::style;
use similar::{ChangeTag, TextDiff};

pub struct DiffSummary {
    pub additions: usize,
    pub deletions: usize,
}

pub fn print_unified_diff(file_path: &str, old_content: &str, new_content: &str) -> DiffSummary {
    let diff = TextDiff::from_lines(old_content, new_content);
    let mut additions = 0;
    let mut deletions = 0;

    println!(
        "\n{} {}",
        style("DIFF").bold().black().on_yellow(),
        style(file_path).bold().underlined()
    );
    println!("{}", style(format!("--- a/{}", file_path)).red());
    println!("{}", style(format!("+++ b/{}", file_path)).green());

    for hunk in diff.unified_diff().iter_hunks() {
        println!("{}", style(hunk.header()).cyan());
        for change in hunk.iter_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    deletions += 1;
                    print!("{}", style(format!("-{}", change.value())).red());
                }
                ChangeTag::Insert => {
                    additions += 1;
                    print!("{}", style(format!("+{}", change.value())).green());
                }
                ChangeTag::Equal => {
                    print!("{}", style(format!(" {}", change.value())).dim());
                }
            }
        }
    }

    println!(
        "{} {}, {}",
        style("Summary:").bold(),
        style(format!("+{} additions", additions)).green(),
        style(format!("-{} deletions", deletions)).red()
    );

    DiffSummary {
        additions,
        deletions,
    }
}
