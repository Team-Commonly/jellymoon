//! An agent that does what the interrupt asks. It ends by itself, with a success reported and exit 0, and the run
//! is cancelled all the same: the person asked for the stop while the agent was still going
//! (`docs/development/run.md`, "A cancelled run"). And one that prints past the stream's cap once it is asked.

use super::{ended, printed, said_cancelled, world};

#[test]
fn an_agent_that_reports_a_success_and_exits_0_at_the_interrupt_is_cancelled_with_no_star() {
    // The result before the interrupt (0), and at it (1).
    for number in [0, 1] {
        let w = world();
        let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
        // Fifteen seconds after each asking: the agent is gone long before, by itself.
        let Some((going, run)) = w.minded(("obeys", number), 15_000, &wish, &repo) else { return };
        let stopped = w.stop(&wish);
        assert_eq!(printed(&stopped), said_cancelled(&run, &wish, 0), "{number}");
        assert_eq!(printed(&ended(going)), said_cancelled(&run, &wish, 0), "{number}");
        assert!(w.rec("heard").is_some() && w.agent_came_and_went(), "{number}: the agent did not end by itself");
        // No star, the copy kept, and nothing committed on the run's branch.
        w.is_cancelled(&wish, &run, &repo, 0);
        assert_eq!((w.copies(), w.starts()), (vec![run], 1), "{number}");
        w.never_says_the_agent(&stopped);
    }
}
