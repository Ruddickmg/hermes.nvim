pub fn map_codes(exit_code: u32) -> Option<String> {
    Some(match exit_code {
        1 => "SIGHUP",
        2 => "SIGINT",
        3 => "SIGQUIT",
        4 => "SIGILL",
        5 => "SIGTRAP",
        6 => "SIGABRT",
        7 => "SIGBUS",
        8 => "SIGFPE",
        9 => "SIGKILL",
        10 => "SIGUSR1",
        11 => "SIGSEGV",
        12 => "SIGUSR2",
        13 => "SIGPIPE",
        14 => "SIGALRM",
        15 => "SIGTERM",
        16 => "SIGSTKFLT",
        17 => "SIGCHLD",
        18 => "SIGCONT",
        19 => "SIGSTOP",
        20 => "SIGTSTP",
        21 => "SIGTTIN",
        22 => "SIGTTOU",
        23 => "SIGURG",
        24 => "SIGXCPU",
        25 => "SIGXFSZ",
        26 => "SIGVTALRM",
        27 => "SIGPROF",
        28 => "SIGWINCH",
        29 => "SIGIO",
        30 => "SIGPWR",
        31 => "SIGSYS",
        _ => return None
    }.to_string())
}

pub fn map_exit_code_to_signal(exit_code: i64) -> String {
    let formatted = format!("UNKNOWN({})", exit_code);
    if exit_code < 0 {
        map_codes((-exit_code) as u32).unwrap_or(formatted)
    } else if (120 ..255).contains(&exit_code) {
        map_codes(exit_code as u32 - 128).unwrap_or(formatted)
    } else {
        formatted
    }
}
