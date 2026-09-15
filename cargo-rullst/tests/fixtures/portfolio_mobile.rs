#[test]
fn rendered_portfolio_covers_long_content_and_mobile_browser_when_requested() {
    let long = "LongUnbrokenPortfolioContent".repeat(12);
    let profile = Profile {
        id: 1,
        name: long.clone(), title: long.clone(), subtitle: long.clone(),
        email: format!("{long}@example.test"), website: format!("https://example.test/{long}"),
        avatar_url: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='100' height='100'%3E%3C/svg%3E".into(),
        github_url: format!("https://example.test/{long}"),
        linkedin_url: format!("https://example.test/{long}"),
    };
    let project = Project {
        id: 1,
        title: long.clone(),
        description: long.clone(),
        url: "https://example.test/project".into(),
        tags: long.clone(),
        is_featured: 1,
    };
    let experience = Experience {
        id: 1,
        role: long.clone(),
        company: long.clone(),
        period: long.clone(),
        description: long.clone(),
    };
    let skill = Skill {
        id: 1,
        name: long.clone(),
        category: long.clone(),
    };
    let html = render(&profile, &[project], &[experience], &[skill]);
    assert!(html.contains(&long));
    assert!(html.contains("Projects Showcase"));
    assert!(html.contains("/nexus"));
    check_mobile_ui("portfolio", &html);
}
