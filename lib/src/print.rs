use console::{Emoji, style};

use crate::STDOUT_WRITE;

pub fn print_error(message: &str) {
    if unsafe { STDOUT_WRITE } {
        eprintln!("Error: {}", message);
    }
}

pub fn p_error(msg: &str, etype: &str) {
    if !unsafe { STDOUT_WRITE } {
        return;
    }
    eprintln!(
        "{} {}: {}\n\n{}\n",
        Emoji("❗", "!"),
        style("Error: ").bold().underlined().red(),
        etype,
        style(msg),
    );
}

pub fn p_good(msg: &str) {
    if unsafe { !STDOUT_WRITE } {
        return;
    }
    println!("{} {}", Emoji("👍", "✔️"), style(msg).bold().underlined(),);
}

pub fn p_success(msg: &str) {
    if unsafe { !STDOUT_WRITE } {
        return;
    }
    println!("{} {}", Emoji("✅", "✔️"), style(msg).bold().underlined(),);
}
