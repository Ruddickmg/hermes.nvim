fn map_codes(exit_code: u32) -> Option<String> {
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
            // check if it's in the 128+signal range and return any matches, or None
            if (128..=255).contains(&code) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for map_codes function

    #[test]
    fn map_codes_returns_sigterm_for_code_15() {
        assert_eq!(map_codes(15), Some("SIGTERM".to_string()));
    }

    #[test]
    fn map_codes_returns_sigkill_for_code_9() {
        assert_eq!(map_codes(9), Some("SIGKILL".to_string()));
    }

    #[test]
    fn map_codes_returns_sigint_for_code_2() {
        assert_eq!(map_codes(2), Some("SIGINT".to_string()));
    }

    #[test]
    fn map_codes_returns_none_for_unknown_code() {
        assert_eq!(map_codes(999), None);
    }

    #[test]
    fn map_codes_returns_none_for_code_0() {
        assert_eq!(map_codes(0), None);
    }

    #[test]
    fn map_codes_returns_none_for_code_32() {
        assert_eq!(map_codes(32), None);
    }

    #[test]
    fn map_codes_handles_all_standard_signals() {
        // Test all standard Unix signals 1-31 using slice comparison
        let expected: Vec<Option<String>> = vec![
            Some("SIGHUP".to_string()),
            Some("SIGINT".to_string()),
            Some("SIGQUIT".to_string()),
            Some("SIGILL".to_string()),
            Some("SIGTRAP".to_string()),
            Some("SIGABRT".to_string()),
            Some("SIGBUS".to_string()),
            Some("SIGFPE".to_string()),
            Some("SIGKILL".to_string()),
            Some("SIGUSR1".to_string()),
            Some("SIGSEGV".to_string()),
            Some("SIGUSR2".to_string()),
            Some("SIGPIPE".to_string()),
            Some("SIGALRM".to_string()),
            Some("SIGTERM".to_string()),
            Some("SIGSTKFLT".to_string()),
            Some("SIGCHLD".to_string()),
            Some("SIGCONT".to_string()),
            Some("SIGSTOP".to_string()),
            Some("SIGTSTP".to_string()),
            Some("SIGTTIN".to_string()),
            Some("SIGTTOU".to_string()),
            Some("SIGURG".to_string()),
            Some("SIGXCPU".to_string()),
            Some("SIGXFSZ".to_string()),
            Some("SIGVTALRM".to_string()),
            Some("SIGPROF".to_string()),
            Some("SIGWINCH".to_string()),
            Some("SIGIO".to_string()),
            Some("SIGPWR".to_string()),
            Some("SIGSYS".to_string()),
        ];

        let actual: Vec<Option<String>> = (1..=31).map(|code| map_codes(code)).collect();
        assert_eq!(actual, expected);
    }

    // Tests for parse_exit_code function

    #[test]
    fn parse_exit_code_returns_exit_code_and_none_for_normal_exit() {
        // Exit code 42 (normal range 0-127)
        assert_eq!(parse_exit_code(42), (Some(42), None));
    }

    #[test]
    fn parse_exit_code_returns_exit_code_and_none_for_exit_code_0() {
        assert_eq!(parse_exit_code(0), (Some(0), None));
    }

    #[test]
    fn parse_exit_code_returns_exit_code_and_none_for_exit_code_1() {
        assert_eq!(parse_exit_code(1), (Some(1), None));
    }

    #[test]
    fn parse_exit_code_returns_exit_code_and_none_for_exit_code_127() {
        // Max normal exit code
        assert_eq!(parse_exit_code(127), (Some(127), None));
    }

    #[test]
    fn parse_exit_code_returns_exit_code_and_signal_for_128_plus_range() {
        // 137 = 128 + 9 = SIGKILL
        assert_eq!(
            parse_exit_code(137),
            (Some(137), Some("SIGKILL".to_string()))
        );
    }

    #[test]
    fn parse_exit_code_returns_exit_code_and_signal_for_130() {
        // 130 = 128 + 2 = SIGINT
        assert_eq!(
            parse_exit_code(130),
            (Some(130), Some("SIGINT".to_string()))
        );
    }

    #[test]
    fn parse_exit_code_returns_exit_code_and_none_for_unknown_signal_in_128_range() {
        // 255 = 128 + 127, 127 is not a standard signal
        assert_eq!(parse_exit_code(255), (Some(255), None));
    }

    #[test]
    fn parse_exit_code_returns_none_and_signal_for_negative_sigterm() {
        // -15 = SIGTERM
        assert_eq!(parse_exit_code(-15), (None, Some("SIGTERM".to_string())));
    }

    #[test]
    fn parse_exit_code_returns_none_and_signal_for_negative_sigkill() {
        // -9 = SIGKILL
        assert_eq!(parse_exit_code(-9), (None, Some("SIGKILL".to_string())));
    }

    #[test]
    fn parse_exit_code_returns_none_and_unknown_for_negative_unknown_signal() {
        // -999 is not a standard signal
        assert_eq!(
            parse_exit_code(-999),
            (None, Some("UNKNOWN(-999)".to_string()))
        );
    }

    #[test]
    fn parse_exit_code_returns_none_and_unknown_for_too_large_number() {
        // u32::MAX + 1 would overflow
        assert_eq!(
            parse_exit_code(4294967296),
            (None, Some("UNKNOWN(4294967296)".to_string()))
        );
    }

    #[test]
    fn parse_exit_code_handles_negative_1() {
        // -1 = SIGHUP
        assert_eq!(parse_exit_code(-1), (None, Some("SIGHUP".to_string())));
    }

    #[test]
    fn parse_exit_code_handles_exit_code_128() {
        // 128 = 128 + 0, signal 0 doesn't exist
        assert_eq!(parse_exit_code(128), (Some(128), None));
    }

    #[test]
    fn parse_exit_code_handles_large_negative_number() {
        // Very large negative number that's still valid i64
        assert_eq!(
            parse_exit_code(-2147483648),
            (None, Some("UNKNOWN(-2147483648)".to_string()))
        );
    }
}
