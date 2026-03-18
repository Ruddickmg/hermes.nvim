pub fn map_codes(exit_code: u32) -> Option<String> {
    Some(
        match exit_code {
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
            _ => return None,
        }
        .to_string(),
    )
}

pub fn parse_exit_code(exit_code: i64) -> (Option<u32>, Option<String>) {
    let formatted = Some(format!("UNKNOWN({})", exit_code));
    if exit_code < 0 {
        // if the number is negative, we can't use it, try and find a match or use unknown with the negative number
        (None, map_codes((-exit_code) as u32).or(formatted))
    } else {
        let unknown: Result<u32, _> = exit_code.try_into();
        // if the code is a valid u32 number
        if let Ok(code) = unknown {
            // check if it's in the other term code range and return any matches, or None
            if (120..255).contains(&code) {
                (Some(code), map_codes(exit_code as u32 - 128))
            // otherwise it's an unknown code, but we can still just pass the raw number
            } else {
                (Some(code), None)
            }
        } else {
            // if it's not a valid u32, mark it as unknown with the number in string format
            (None, formatted)
        }
    }
}
