//! A follow-up of a run that finished and changed nothing (go on from an answer): a person asked a question, and
//! goes on. It forks the run's session, so the answer is known to it, in a copy made where the person's checkout
//! stands now, as one of kept work's is; its work is kept and tossed as any run's; and one that again changed
//! nothing is followed in its turn.

use serde_json::json;

use super::{AS_IT_WAS, Made, followed, forked, given, printed, world};

const NOTHING: &str =
    "run {run} finished and changed nothing: the branch jellymoon/run-{run} is where the checkout was";

/// The wish `m1`, whose run the made-up agent ended changing nothing: a question answered.
fn asked() -> Made {
    let w = world();
    w.play("nothing", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let said = printed(&w.start(&wish, &repo));
    let run = said["run"].as_str().unwrap().to_string();
    assert_eq!(said["why"], NOTHING.replace("{run}", &run));
    Made { w, repo, wish, run }
}

#[test]
fn a_follow_up_of_a_question_forks_its_session_in_a_copy_of_the_checkout_as_it_is_now() {
    for verb in ["keep", "toss"] {
        let made = asked();
        let (w, session, first) = (&made.w, made.w.args().1, made.record());
        // The person changes a file after the question was answered.
        let stands = made.moved_on();
        let (wish, said) = made.follows("follow", 0);
        let run = said["run"].as_str().unwrap().to_string();
        assert_eq!((&said["outcome"], &w.kept(&wish)["after"]), (&json!("completed"), &json!("m1")), "{said}");
        let (record, (args, found)) = (w.record(&run), given(w, 2));
        assert_eq!(args, forked(&session, record["session"].as_str().unwrap()));
        assert_eq!((&record["follows"], &record["forked"]), (&json!(made.run), &json!(true)));
        // Its copy holds the person's change since, and starts where their checkout stands, not at the old commit.
        let held = AS_IT_WAS.replace("c.txt: c.txt as it was", "c.txt: c.txt as the person left it");
        assert_eq!((found, &record["start"]), (held, &json!(stands)));
        assert_ne!(first["end"], json!(stands));
        // Its work is kept, or tossed, as any run's is.
        let done = printed(&made.asked(&[verb], &wish, &[]));
        match verb {
            "keep" => assert_eq!((&done["kept"], &done["onto"]), (&record["end"], &json!("main"))),
            _ => assert_eq!(done["tossed"], true, "{done}"),
        }
    }
}

#[test]
fn three_questions_in_a_row_each_follow_the_last() {
    let made = asked();
    let w = &made.w;
    let (second, said) = made.follows("follow", 2);
    let run = said["run"].as_str().unwrap().to_string();
    assert_eq!(w.record(&run)["start"], made.record()["end"], "it changed nothing either");
    let third = followed(w, &second);
    w.play("follow", 2);
    let said = printed(&made.asked(&[], &third, &[]));
    let record = w.record(said["run"].as_str().unwrap());
    assert_eq!((&said["outcome"], &record["follows"]), (&json!("completed"), &json!(run)));
    let session = w.record(&run)["session"].as_str().unwrap().to_string();
    assert_eq!(given(w, 3).0, forked(&session, record["session"].as_str().unwrap()));
}
