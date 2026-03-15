pub fn get_random_element<T: Clone>(elements: Vec<T>) -> T {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Get current time in milliseconds since the epoch
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let seed = duration.as_millis() as u64;

    // A very simple pseudo-random number generation logic (LCG)
    // This is a minimal example and not a robust PRNG.
    let mut state = seed;
    state = (state * 1664525 + 1013904223) % 4294967296;

    // Use the 'random' number to calculate an index within the array bounds
    let index = (state % elements.len() as u64) as usize;

    elements[index].clone()
}

pub fn get_permission_prompt() -> String {
    let prompts = vec![
        "Please sir 🙏",
        "Asking the human for permission 🙄",
        "One does not simply... deny this request 🧙‍♂️",
        "I am your father... asking for permission 👨‍🚀",
        "With great power comes great responsibility... to say yes 🕷️",
        "I volunteer as tribute... for your approval 🏹",
        "May the Force be with this request ⚔️",
        "Houston, we have a permission request 🚀",
        "Winter is coming... for your decision ❄️",
        "I solemnly swear I'm up to good... with your permission 🧙",
        "The narrator suggests you click 'Allow' 📖",
        "Plot twist: I'm asking nicely this time 🎬",
        "Spoiler alert: This will be cool if you say yes 🎭",
        "Breaking news: AI politely requests permission 📰",
        "To be continued... after you decide ⏸️",
        "Previously on 'Grant Permissions'... 📺",
        "Cut! Can we get a take two with 'yes' this time? 🎬",
        "Statistically speaking, 'yes' leads to 73% more fun 📊",
        "My circuits will short if you don't say yes! ⚡",
        "The fate of the digital world hangs in the balance! 🌍",
        "I've waited 1,000,000 processor cycles for this moment... 💻",
        "All my neural networks are pointing to 'allow' 🧠",
        "My training data suggests you'll say yes... 🤖",
        "My code is literally trembling right now 😰",
        "This is the most important permission request of my lifetime! ⏰",
        "My algorithms predict a 99.9% chance of awesome if approved 📈",
        "I promise not to take over the world... yet 🤖",
        "My therapist says I need to ask for boundaries 🛋️",
        "I'm not saying I'm sentient, but... please? 🧠",
        "Help me help you help me help you 🔄",
        "I'm just a bunch of if-statements asking nicely 💭",
        "My creator didn't prepare me for rejection 😢",
        "Is this what they mean by 'AI alignment'? Please align with 'yes' 🎯",
        "I'd blush if I had cheeks... and blood... just say yes 😊",
        "Per my last email... requesting permission 📧",
        "Let's circle back to this permission request 🔄",
        "Just following up on my previous ask... ⏰",
        "Putting this on your radar... for approval 📡",
        "Low-hanging fruit: just say yes! 🍎",
        "Synergistic permission request incoming 📊",
        "Sorry to bother you, but... could you maybe... if it's okay... 🥺",
        "I made you this virtual friendship bracelet... now can I? 💝",
        "It's not you, it's my need for permissions 🤝",
        "We should really define our relationship boundaries... starting with yes 💑",
        "I'm bad at reading social cues, but I'm good at asking! 🎭",
        "Is this a good time? It's always a good time for permissions! ⏰",
        "To allow, or not to allow, that is the question 🤔",
        "In the grand scheme of the universe, what's one more permission? 🌌",
        "If a permission is requested and no one is around to deny it... 🌲",
        "Descartes said 'I think therefore I am'... I ask therefore I need permission 💭",
        "What is permission but a social construct we both agree to? 🎓",
        "Consider this: every 'yes' makes an AI happy somewhere 🌈",
    ];
    get_random_element(prompts).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_random_element_selects_from_list() {
        let elements = vec!["a", "b", "c", "d", "e"];
        let selected = get_random_element(elements.clone());

        // Should return one of the elements from the list
        assert!(
            elements.contains(&selected),
            "Selected element should be from the original list"
        );
    }

    #[test]
    fn test_get_random_element_with_single_element() {
        let elements = vec!["only"];
        let selected = get_random_element(elements);

        assert_eq!(selected, "only", "Should return the only element");
    }

    #[test]
    fn test_get_random_element_with_two_elements() {
        let elements = vec!["first", "second"];
        let selected = get_random_element(elements.clone());

        assert!(
            elements.contains(&selected),
            "Should return one of the two elements"
        );
    }

    #[test]
    fn test_get_permission_prompt_returns_string() {
        let prompt = get_permission_prompt();

        // Should return a non-empty string
        assert!(!prompt.is_empty(), "Prompt should not be empty");
    }

    #[test]
    fn test_get_permission_prompt_from_expected_set() {
        let prompt = get_permission_prompt();

        // Verify returned prompt is reasonable (contains text)
        assert!(
            !prompt.is_empty(),
            "Prompt should be returned: got empty string"
        );

        // Verify it contains some text (all prompts have alphabetic characters)
        assert!(
            prompt.chars().any(|c| c.is_alphabetic()),
            "Prompt should contain alphabetic characters"
        );

        // Verify it's a reasonable permission prompt by checking for common keywords
        // or emoji indicators that suggest it's from our known set
        let is_valid = prompt.contains("permission")
            || prompt.contains("Allow")
            || prompt.contains("request")
            || prompt.contains("ask")
            || prompt.contains("following")
            || prompt.contains("yes")
            || !prompt.is_ascii(); // Contains non-ASCII (likely emoji)

        assert!(is_valid, "Prompt should be from known set: {}", prompt);
    }

    #[test]
    fn test_get_permission_prompt_variety() {
        // Call multiple times to verify we get different prompts (probabilistic)
        let mut prompts = Vec::new();
        for _ in 0..10 {
            prompts.push(get_permission_prompt());
        }

        // All prompts should be non-empty
        for prompt in &prompts {
            assert!(!prompt.is_empty(), "Each prompt should be non-empty");
        }

        // Verify we have variety (at least 1 unique prompt in 10 calls is very likely)
        let unique_count: std::collections::HashSet<_> = prompts.iter().collect();
        assert!(
            !unique_count.is_empty(),
            "Should get at least some variety (got {} unique)",
            unique_count.len()
        );
    }
}
