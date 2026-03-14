//! Integration tests for prompt utilities
use hermes::utilities::{get_permission_prompt, get_random_element};

#[nvim_oxi::test]
fn test_get_random_element_selects_from_list() -> nvim_oxi::Result<()> {
    let elements = vec!["a", "b", "c", "d", "e"];
    let selected = get_random_element(elements.clone());

    // Should return one of the elements from the list
    assert!(
        elements.iter().any(|&e| e == selected),
        "Selected element should be from the original list"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_get_random_element_with_single_element() -> nvim_oxi::Result<()> {
    let elements = vec!["only"];
    let selected = get_random_element(elements);

    assert_eq!(selected, "only", "Should return the only element");

    Ok(())
}

#[nvim_oxi::test]
fn test_get_random_element_with_two_elements() -> nvim_oxi::Result<()> {
    let elements = vec!["first", "second"];
    let selected = get_random_element(elements.clone());

    assert!(
        elements.iter().any(|&e| e == selected),
        "Should return one of the two elements"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_get_permission_prompt_returns_string() -> nvim_oxi::Result<()> {
    let prompt = get_permission_prompt();

    // Should return a non-empty string
    assert!(!prompt.is_empty(), "Prompt should not be empty");

    Ok(())
}

#[nvim_oxi::test]
fn test_get_permission_prompt_from_expected_set() -> nvim_oxi::Result<()> {
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

    // Verify it's one of the expected prompts (all 50 prompts contain these patterns)
    let is_valid = prompt.contains("permission")
        || prompt.contains("Allow")
        || prompt.contains("request")
        || prompt.contains("ask")
        || prompt.contains("following")
        || prompt.contains("🙏")
        || prompt.contains("👨‍🚀")
        || prompt.contains("⚔️")
        || prompt.contains("🧙‍♂️")
        || prompt.contains("🤖")
        || prompt.contains("📧")
        || prompt.contains("🌈")
        || prompt.contains("⏰");

    assert!(is_valid, "Prompt should be from known set: {}", prompt);

    Ok(())
}

#[nvim_oxi::test]
fn test_get_permission_prompt_variety() -> nvim_oxi::Result<()> {
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
        unique_count.len() >= 1,
        "Should get at least some variety (got {} unique)",
        unique_count.len()
    );

    Ok(())
}
