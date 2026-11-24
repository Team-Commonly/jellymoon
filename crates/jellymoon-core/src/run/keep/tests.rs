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
        (judge(files, Yes::Said(with), true, &mut settings), settings)
    };
    assert_eq!(asked(&[file("src/a.rs", false)], false), (Ok(()), vec![]));
    let obeyed = [file("src/a.rs", false), file(".claude/settings.json", false), file("CLAUDE.md", false)];
    let names = vec![".claude/settings.json".to_string(), "CLAUDE.md".to_string()];
    assert_eq!(asked(&obeyed, false), (Err(Unkept::Settings), names.clone()));
    assert_eq!(asked(&obeyed, true), (Ok(()), names.clone()));
    // A page's keep lands the ones the person was named and no other: every one of the work must be among them.
    let none = std::cell::Cell::new(None);
    let to = |named: &[String], works: bool| judge(&obeyed, Yes::To(named, &none), works, &mut Vec::new());
    for works in [true, false] {
        assert_eq!(
            (to(&names, works), to(&names[..1], works), to(&names[1..], works), to(&[], works)),
            (Ok(()), Err(Unkept::Settings), Err(Unkept::Settings), Err(Unkept::Settings))
        );
    }
    // And every name must be one of the work's: a yes to a file past the first that the work has not is no yes
    // to this work. What one new commit would land of the work may be fewer files than the work's.
    let (mut other, mut swapped) = (names.clone(), names.clone());
    other.push(".envrc".to_string());
    swapped[1] = "AGENTS.md".to_string();
    assert_eq!((to(&other, true), to(&other, false)), (Err(Unkept::Settings), Ok(())));
    assert_eq!((to(&swapped, true), to(&swapped, false)), (Err(Unkept::Settings), Err(Unkept::Settings)));
    let plain = [file("src/a.rs", false)];
    assert_eq!(judge(&plain, Yes::To(&[], &none), true, &mut Vec::new()), Ok(()));
    assert_eq!(judge(&plain, Yes::To(&names[..1], &none), true, &mut Vec::new()), Err(Unkept::Settings));
    // The names are a set: as many as a work may have are asked of in one pass.
    let many: Vec<String> = (0..20_000).map(|n| format!("d{n}/CLAUDE.md")).collect();
    let files: Vec<Touched> = many.iter().map(|name| file(name, false)).collect();
    assert_eq!(judge(&files, Yes::To(&many, &none), true, &mut Vec::new()), Ok(()));
    assert_eq!(judge(&files, Yes::To(&many[1..], &none), true, &mut Vec::new()), Err(Unkept::Settings));
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

#[test]
fn a_records_reason_is_a_name_and_a_record_with_none_or_with_one_unknown_reads_as_none() {
    let start = r#"{"wish":"m1","day":"2026-03-10","started":1,"outcome":"completed","denied":0,"skipped":0"#;
    let read = |more: &str| serde_json::from_str::<record::Record>(&format!("{start}{more}}}")).unwrap();
    // A record from before the reason was written, and one whose change landed, hold no such word.
    let mut record = read("");
    assert_eq!(record.unkept, None);
    assert!(!serde_json::to_string(&record).unwrap().contains("unkept"));
    assert_eq!(read(r#","unkept":"clash""#).unkept, Some(Unkept::Clash));
    assert_eq!(read(r#","unkept":{"unread":"moved"}"#).unkept, Some(Unkept::Unread(copy::Unread::Moved)));
    // A name a later build wrote, or what is no name at all, is no reason here, and the record is still read.
    for unknown in [r#""a_later_reason""#, r#""Clash""#, "7", "null", r#"{"unread":"later"}"#, r#"["clash"]"#] {
        assert_eq!(read(&format!(r#","unkept":{unknown}"#)).unkept, None, "{unknown}");
    }
    // It is written as that name and nothing else: no sentence is kept.
    record.unkept = Some(Unkept::InWay);
    assert!(serde_json::to_string(&record).unwrap().contains(r#""unkept":"in_way""#));
}
