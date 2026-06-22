North Star

Reliable software builds trust — trust is our biggest asset. Every principle below serves that goal.



* Always apply: logging, lessons, README, KISS, POLA.

* Full path adds: TDD, full layered modeling, database context folder, formal specs and plans, SCD/grain discipline, schema-change management.

* The path is chosen explicitly during ideation, not by drift.



How we think



* Long-term thinking: we design for continuity, not just the next sprint.

* Avoid unnecessary work: every task gets a trade-off analysis before we commit.

* Right-sized effort: small projects don't carry big-project overhead. Apply these principles proportionally. 

* We will not apply rocket science to a a simple project, that is using a chainsaw to slice a piece of cake.

* Pragmatic but informed: we consult data engineering experts and watch industry standards without chasing every trend. If the new trend is a clear winner for a specific project, we will pursue it.



How we code



* TDD with intent: tests are written first, deliberately, well thought behind. They encode what the code is for, not just that it runs. TDD is our default — only throwaway exploration is exempt, and AI makes full TDD viable everywhere else.

* DRY, achieved through patient abstraction — never premature.

* AHA (Avoid Hasty Abstractions): when in doubt, prefer duplication over the wrong abstraction. Abstraction can make things complicated.

* KISS: simple where simplicity doesn't sacrifice correctness or completeness.

* POLA (Principle of Least Astonishment): code behaves the way a reader would expect.

* Command-Query Separation: a method either changes state or returns data — never both.

* Idempotency: is crucial when syncing and versioning features are involved.





How we observe



* Logging is non-negotiable. A junior dev with zero project context should be able to follow the project from logs alone.

* We have a exposed Logs folder where we can share the logs to the dev env. Depending on the project, we will have more or less sophistication for this approach.



How we document



* README for newcomers: any reader grasps the project in 2 minutes.

* For dummies: project for dummies so a person can quickly understand the entire project and how it is connected. Each data schema for dummies inside the database context folder, so it is also easy to grasp, and if one wants details goes to the detail files in the context.

* Database context folder: data model, structural decisions, script run history — kept current so connections and runbooks can always be reconstructed. For dummies files here too.

* Runbooks for recurring operational fixes.

* Every plan, specs, steering or consulting decisions are saved in files in our docs.

* Structure: docs/MADRs for ADRs, docs/lessons for lessons not to repeat, docs/steering for the actual roadmap and vision, docs/plans the plan to make those features or dreams into hig-quality code, docs/specs feature specifications created after questions and different users sessions



How we learn



* Lessons folder: every mistake and experiment failed becomes part of a Markdown file, we have a markdown file for different types of mistakes. We pay for a lesson once! Those are written into docs/lessons.

* ADRs: we log our decisions following the MADR convention. We save them with clear naming and structure in docs/MADRs



How we deliver



1. Ideation — explore the problem, build POCs. The tech stack of any project is decided while building POCs and consultative job with different type of experts (pragmatic, conservative, principles driven, and new tech adopter).

1.5. Road Map vision — this allows us to high level what we want to obtain but not the how

2. Knowledge gathering — decide which POC findings are worth promoting.

3. Specs — written before plans.

4. Plan — derived from specs, saved as MarkDown, saved in a folder with all plans in the project. We have a Beads also that carries the plan.

5. Build.

