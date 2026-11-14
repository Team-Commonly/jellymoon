use super::*;

#[test]
fn what_a_keep_wrote_in_a_record_is_an_id_and_a_branchs_whole_name_or_the_record_is_not_cores() {
    let id = "a".repeat(40);
    let named = |kept: &str, onto: &str| intent(&Some(kept.to_string()), &Some(onto.to_string())).map(|_| ());
    assert_eq!(intent(&None, &None), Ok(None));
    assert_eq!(named(&id, "refs/heads/main"), Ok(()));
    assert_eq!(named(&id, "refs/heads/feature/x-1"), Ok(()));
    // The one word of work that landed on no branch, and names of other alphabets and of 1024 bytes.
    for onto in ["HEAD", "refs/heads/ok-ünï", "refs/heads/@", &format!("refs/heads/{}", "x".repeat(1013))] {
        assert_eq!(named(&id, onto), Ok(()), "{onto}");
    }
    for onto in [
        "refs/heads/a\u{a0}b",
        "refs/heads/a\u{3000}b",
        "refs/heads/a\u{2028}b",
        "refs/heads/a\u{85}b",
        "refs/heads/a\u{9b}b",
        &format!("refs/heads/{}", "x".repeat(1014)),
        "refs/heads/HEAD ",
        "head",
        "main",
        "refs/tags/v1",
        "refs/heads/a b",
        "refs/heads/*",
        "refs/heads/a\nb",
        "refs/heads/a?",
        "refs/heads/[a]",
        // A run's branch is never where work was kept.
        "refs/heads/jellymoon/run-r0123456789abcdef",
        "refs/heads/jellymoon/run-",
        // Nor is another spelling of one: a disk that folds case takes it for the run's branch.
        "refs/heads/Jellymoon/run-r0123456789abcdef",
        "refs/heads/JELLYMOON/RUN-x",
    ] {
        assert_eq!(named(&id, onto), Err(()), "{onto}");
    }
    for kept in ["main", "HEAD", "--all", &id[..39]] {
        assert_eq!(named(kept, "refs/heads/main"), Err(()), "{kept}");
    }
    assert_eq!(intent(&Some(id), &None), Err(()));
    assert_eq!(intent(&None, &Some("refs/heads/main".to_string())), Err(()));
}

#[test]
fn another_repos_entry_is_never_kept_and_a_file_tools_obey_only_when_asked() {
    let file = |path: &str, repo: bool| Touched::of(path.as_bytes(), repo);
    let asked = |files: &[Touched], with: bool| {
        let mut settings = Vec::new();
        (judge(files, Yes::Said(with), &mut settings), settings)
    };
    assert_eq!(asked(&[file("src/a.rs", false)], false), (Ok(()), vec![]));
    let obeyed = [file("src/a.rs", false), file(".claude/settings.json", false), file("CLAUDE.md", false)];
    let names = vec![".claude/settings.json".to_string(), "CLAUDE.md".to_string()];
    assert_eq!(asked(&obeyed, false), (Err(Unkept::Settings), names.clone()));
    assert_eq!(asked(&obeyed, true), (Ok(()), names.clone()));
    // A page's keep lands the ones the person was named and no other: every one of the work must be among them.
    let to = |named: &[String]| judge(&obeyed, Yes::To(named), &mut Vec::new());
    assert_eq!(
        (to(&names), to(&names[..1]), to(&names[1..]), to(&[])),
        (Ok(()), Err(Unkept::Settings), Err(Unkept::Settings), Err(Unkept::Settings))
    );
    assert_eq!(judge(&[file("src/a.rs", false)], Yes::To(&[]), &mut Vec::new()), Ok(()));
    // Another repo's entry is refused whatever was asked for, and before a file tools obey is.
    for repos in
        [file("sub", true), file(".gitmodules", false), file("x/.GITMODULES", false), file(".gitmodules ", false)]
    {
        let files = [file("CLAUDE.md", false), repos];
        assert_eq!(asked(&files, true).0, Err(Unkept::Repos));
        assert_eq!(asked(&files, false).0, Err(Unkept::Repos));
    }
    // A character nobody sees does not hide a name: the name is asked as it is printed too.
    assert_eq!(asked(&[file(".git\u{200b}modules", false)], true).0, Err(Unkept::Repos));
}
