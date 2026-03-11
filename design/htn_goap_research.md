# Hierarchical Task Networks and Goal Oriented Action Planning for Modern Agentic Systems

## Executive summary

Hierarchical Task Network (HTN) planning and Goal-Oriented Action Planning (GOAP) are two distinct but related approaches to deliberative action selection and multi-step behavior synthesis. GOAP, as used in game AI, is primarily an adaptation of STRIPS-style classical planning: it represents world states as sets of propositional facts, represents actions with preconditions/effects and costs, and uses search (commonly A* or related techniques) to find a low-cost action sequence to reach a desired goal condition. citeturn2search0turn2search13turn12view2turn14view0

HTN planning is a hierarchical refinement paradigm: it represents “what to do” initially as abstract tasks and repeatedly decomposes them into more concrete subtasks via methods until primitive (directly executable) actions remain, subject to ordering and other constraints. This introduces an explicit engineering layer for hierarchical operational knowledge; in many practical settings this constrains the plan space and can reduce search relative to unconstrained goal-state planning, while also creating distinct correctness/verification questions and potential expressivity/decidability issues depending on the HTN fragment. citeturn0search3turn9search5turn1search12turn7search0

The most practically relevant differences for software architecture work are: (a) whether the plan space is *goal-defined* (GOAP) versus *procedure-constrained* (HTN); (b) whether hierarchical control knowledge is explicit and mandatory (HTN) versus optional (GOAP, via heuristics/costs); (c) how execution monitoring and repair are handled, where HTN planning has a mature literature on plan repair/replanning with task hierarchy constraints and on integrating acting/planning over shared hierarchical operational models. citeturn7search1turn7search3turn7search23

In modern non-game applications, HTN planning appears in domains such as web service composition, human-assistance systems that plan/repair/execute/explain, and robotics task planning integrated with acting and (in some lines) with motion planning. citeturn0search19turn7search2turn9search2turn7search15

In agentic execution platforms (tool-using agents, long-horizon workflows, coding agents), both HTN and GOAP require explicit treatment of (i) partial observability and stale state, (ii) nondeterministic outcomes and expensive actions, (iii) side effects and idempotency, (iv) persistence of execution traces and artifact lineage. The relevant mature bodies of work are (a) planning under uncertainty (e.g., FOND and contingent planning) and its hierarchical extensions, and (b) workflow/durable execution patterns (event histories, idempotency, sagas, provenance standards). citeturn10search4turn1search5turn3search7turn5search4turn5search3turn5search33

## Terminology and concept glossary

This glossary is scoped to the specific deliverables requested, and uses established formal planning conventions.

**World state (state)**  
A representation of the environment at a time point used for planning and execution. In STRIPS-style planning, a state is often represented as a set (conjunction) of ground atoms/literals (facts) taken as true; actions transform one state into another via add/delete (or more general) effects. citeturn2search0turn12view2turn14view0

**Action / operator (primitive action)**  
A directly executable step with:
- **Preconditions:** conditions that must hold in the current state for the action to be applicable.
- **Effects:** how the action changes the state (classically as add/delete effects; often extended with conditional effects, numeric/temporal effects, etc., in richer languages). citeturn2search0turn2search3turn14view0

**Plan (classical plan)**  
A sequence or partial order of actions that, when executed from an initial state (according to the action semantics), results in a state satisfying a goal condition. citeturn2search0turn9search3

**Goal condition**  
A logical condition over states that must be satisfied by a plan’s final state (or by a policy’s terminal states in nondeterministic settings). citeturn2search0turn10search4

**Execution trace**  
A record of executed actions (and often observed states/observations) with timestamps/outcomes; used for monitoring, debugging, and repair. In workflow systems, analogous constructs include event histories used for recovery and replay. citeturn3search7turn5search6turn5search2

### HTN-specific terms

**Task**  
An activity to be accomplished. Tasks are usually represented syntactically like predicates/atoms with arguments (parameters), similar to actions, but differ semantically depending on whether they are primitive or compound. citeturn9search5turn1search12turn15search6

**Primitive task**  
A task that corresponds to a directly executable action/operator (with state-transition semantics). citeturn0search3turn9search5turn9search28

**Compound (abstract) task**  
A task that is not directly executable and must be refined/decomposed into subtasks using a method. citeturn9search5turn1search12turn15search6

**Task network**  
A (typically partially ordered) set of task occurrences plus constraints (e.g., ordering constraints). Many HTN formalisms work with partially ordered task networks; some restrict to totally ordered networks for tractability or tooling reasons. citeturn0search3turn1search12turn16search6

**Method**  
A decomposition rule that can replace a compound task (or a task occurrence in a task network) with a sub-network of tasks, optionally guarded by method preconditions/constraints. citeturn9search5turn1search12turn15search6

**Decomposition (refinement)**  
The operation of applying a method to a compound task, thereby introducing its subtasks and constraints into the current task network (and removing or expanding the refined task). citeturn9search5turn1search12turn9search35

**HTN plan (solution concept)**  
A plan is typically a sequence (or a partially ordered set) of primitive tasks/actions that can be obtained via repeated decomposition/refinement of the initial task network and that is executable from the initial state. Exact solution definitions vary across HTN variants and restrictions, and directly affect decidability/complexity. citeturn0search3turn1search12turn9search5turn1search1

**Replanning / repair in HTN**  
Replanning is generating a new plan from an updated state; HTN repair must also account for hierarchical constraints and for already executed actions/tasks. Work in HTN plan repair notes that naive discarding/replanning can be nontrivial because the solution criteria constrain how execution history fits the remaining task network. citeturn7search1turn7search23turn7search25

### GOAP-specific terms

**GOAP world state representation**  
In GOAP as described for game AI, the current world state is represented as a conjunction (set) of literals/facts (or equivalently as assignments to variables describing the world). citeturn11view0turn12view2turn14view0

**Goal selection**  
A mechanism that chooses which goal to pursue given current world state and competing objectives. In GOAP game AI, goals are often prioritized or weighted (e.g., “insistence”/priority) and then the planner generates a plan for the selected goal. citeturn12view1turn12view2turn14view0

**Action modeling in GOAP**  
Actions are modeled with preconditions and effects (often STRIPS-like), plus a cost used for least-cost planning. Some GOAP implementations also incorporate “procedural” preconditions/effects for runtime practicality. citeturn12view2turn16search27turn14view0

**Heuristic choice**  
GOAP planning frequently uses A* or related search methods where the heuristic estimates remaining cost-to-go, analogous to pathfinding. This is a transfer of standard heuristic search theory to the action-planning graph. citeturn6search0turn12view2turn14view0

**World state progression**  
Applying an action to a state yields a successor state via the action’s effects (classically: remove negative effects, add positive effects). citeturn14view0turn2search0

**Execution loop (plan-act-monitor-replan)**  
A standard GOAP control loop in games is: sense or update world state; select a goal; plan a sequence of actions; execute stepwise; replan when the plan becomes invalid or higher-priority goals arise. The motivating examples in game practice explicitly emphasize replanning when actions fail (e.g., door blocked ⇒ replan to kick door ⇒ replan to alternate entry). citeturn12view2turn11view0

## Historical timeline

This timeline prioritizes milestones that shaped today’s engineering practice and tool ecosystems.

**1970s**  
- STRIPS introduced a planning framework based on searching sequences of operators to transform an initial world model into one that satisfies a goal condition; it formalized operator preconditions and effects as central constructs. citeturn2search0turn2search13  
- A* established a formal basis for heuristic minimum-cost path search with evaluation functions of the form *f(n)=g(n)+h(n)*; GOAP’s common use of A*-like search for action sequences is an application of this framework to the action space rather than navigation graphs. citeturn6search0turn12view2

**1990s**  
- HTN planning complexity/expressivity results formalized that the complexity of HTN plan existence varies widely depending on restrictions on task networks and methods; this period also includes formal syntax/semantics and sound/complete procedures for specific HTN fragments (e.g., UMCP). citeturn0search3turn9search5turn0search18  
- Graphplan introduced planning graphs and a planning procedure that returns a shortest partial-order plan or reports no plan; it influenced later heuristic planning and planning-graph-based heuristics. citeturn9search3turn9search22  
- PDDL appeared to standardize planning domain/problem descriptions for competitions and comparative evaluation, later extended (e.g., PDDL2.1) to represent temporal/numeric properties. citeturn2search6turn2search11turn2search3

**Early 2000s**  
- SHOP2 described an HTN planner that generates plan steps in the order they will be executed, enabling the planner to track the current state during planning and supporting expressive temporal/metric domains. citeturn0search4turn0search16  
- HTN planning was applied to semantic web service composition (composition of OWL-S services), illustrating non-game applications where hierarchical decomposition aligns with structured processes. citeturn0search19

**Mid 2000s**  
- GOAP became a widely cited game AI interpretation of planning, exemplified by the “Three States and a Plan” approach for the video game entity["video_game","F.E.A.R.","fps 2005"], emphasizing decoupling goals and actions, layering behaviors, and dynamic problem solving via replanning with a STRIPS-like representation and A*-style search. citeturn12view0turn12view2turn12view1

**2010s**  
- Hierarchical planning integrated with robotics task-and-motion planning, exemplified by “Hierarchical Task and Motion Planning in the Now,” emphasizing aggressive hierarchical commitment to reduce search and limit long-horizon projection, and by subsequent work on integrating acting/planning/learning in hierarchical operational models. citeturn9search2turn7search11turn7search3  
- Task insertion HTN (TIHTN) formalized a hybrid of classical planning and HTN planning by allowing insertion of actions outside the method hierarchy to address incomplete hierarchies and increase flexibility. citeturn1search1turn1search4  
- Behavior Trees expanded from game AI to robotics and AI as a modular control structure for task switching, with formal analysis tools and extensions to stochastic outcomes. citeturn2search4turn2search12

**2020s**  
- HDDL proposed a shared input language for hierarchical planning (an extension of PDDL) to enable domain-independent HTN planning comparisons and integration, and hierarchical planning tracks emerged in the International Planning Competition using HDDL. citeturn16search8turn16search1turn16search5turn16search20  
- Tooling matured around hierarchical planning model authoring/validation (e.g., HDDL Parser as an IDE-integrated language server). citeturn15search5turn15search12  
- Work on HTN plan verification and plan repair expanded, including translations/compilations and parsing-based approaches for verification in (notably) totally ordered HTN fragments. citeturn16search2turn16search6turn7search1  
- Planning under nondeterminism and partial observability continued to mature (FOND, contingent planning) and hierarchical analogues appeared (e.g., “strong solutions for FOND HTN” and probabilistic hierarchical goal networks). citeturn10search12turn10search2turn1search5turn10search4  
- Emerging research interleaves hierarchical symbolic planning with LLM-generated decompositions (e.g., ChatHTN), with explicit claims of soundness while using LLMs as approximate decomposition generators when methods are missing. citeturn15search21turn15search0turn15search2

## HTN deep dive

### Rigorous conceptual model

A common formal view of HTN planning defines a planning problem by:
- an initial world state *s₀*,
- an initial task network (often containing at least one initial abstract task),
- a set of primitive actions/operators with state-transition semantics,
- a set of methods that decompose abstract tasks into networks of subtasks with constraints. citeturn9search5turn0search3turn1search12turn9search35

**Operationally**, HTN planning is a refinement process:
1. Start from the initial task network.
2. Choose a task occurrence to refine.
3. If it is primitive: check applicability in the current state and commit it to the plan (depending on planner style).
4. If it is compound: choose an applicable method and replace/refine the task occurrence with its subtasks and constraints.
5. Continue until only primitive tasks remain and the resulting set/sequence is executable (respecting ordering constraints). citeturn9search5turn1search12turn0search16turn7search33

This operational structure is shared across many planners but parameterized by:
- task network ordering (totally ordered vs partially ordered),
- search strategy (forward progression vs plan-space refinement vs compilations),
- whether and how states are propagated during refinement (e.g., SHOP2’s “planning in the execution order” property). citeturn0search16turn1search18turn1search12turn1search6

### Precise definition set (requested deliverable)

The following definitions use the standard HTN framing in formal treatments and surveys.

**State**  
A set of facts (literals) describing the world. Primitive actions define state transition semantics via preconditions/effects. citeturn2search0turn9search5

**Task**  
A symbolic description of an activity. Tasks can be parameterized and appear as nodes in task networks. citeturn9search5turn1search1

**Primitive task**  
A task corresponding to an action/operator that can directly execute in the environment (subject to preconditions). citeturn0search3turn9search5

**Compound task**  
A task that is not directly executable and must be decomposed using a method. citeturn9search5turn1search12

**Method**  
A rule of the form “to achieve compound task *t*, if method constraints hold, replace *t* with subtask network *N*,” where *N* includes task nodes and constraints (ordering, variable bindings, and often state constraints). citeturn9search5turn1search12turn1search1

**Decomposition (refinement) step**  
Applying a method to expand a compound task into its subtask network, adding constraints and replacing the expanded task occurrence in the current network. citeturn9search5turn1search1

**Precondition / effect (in HTN)**  
- For primitive tasks: preconditions/effects are defined as in STRIPS-style action models. citeturn2search0turn0search16  
- For compound tasks/methods: formalisms vary; some include method preconditions and state constraints that must hold for a decomposition to be applicable (and surveys treat these as significant semantic parameters). citeturn16search6turn1search12turn7search21

**Plan**  
A sequence or partially ordered set of primitive tasks/actions that is obtainable by repeated decomposition from the initial task network and is executable from the initial state, respecting both action applicability and task network constraints. citeturn0search3turn9search5turn1search12

**Execution trace**  
A history including (at minimum) executed primitive actions and observed outcomes/states; used for monitoring, diagnosis, and repair. In HTN-specific plan repair literature, the distinction between executed prefix and remaining hierarchical structure is treated as a central issue. citeturn7search1turn7search25turn7search23

**Replanning and plan repair**  
- **Replanning:** compute a new plan from a new state/problem instance.  
- **Plan repair:** modify an existing plan/hierarchy to accommodate execution failures or model inaccuracies. HTN plan repair work states that discarding and replanning is not “easily possible” in general HTN settings because hierarchical solution criteria require already executed actions to be taken into account, motivating repair approaches and transformations. citeturn7search13turn7search1turn7search23

### Representational consequences and algorithmic tradeoffs

**Expressiveness and complexity**  
Formal results show HTN planning complexity varies with restrictions and can be high; the AAAI-era complexity work explicitly studies how task-network conditions influence complexity. citeturn0search3turn0search24  
Hybrid variants (e.g., task insertion) are analyzed separately, including tight bounds for TIHTN. citeturn1search1turn1search25

**Search space shaping**  
HTN methods encode permissible decompositions, which can function as “search control knowledge” (explicitly recognized in work discussing SHOP2’s performance dependence on hand-designed control knowledge) and can reduce the need to discover structure that is otherwise implicit in goal-state planning. citeturn1search13turn0search16turn1search12

**Planning style variants in HTN systems**  
A survey of hierarchical planning emphasizes that “one abstract idea” has multiple concrete realizations, and modern frameworks (e.g., PANDA) explicitly integrate multiple solving techniques (progression search, plan-space search, compilations) and preprocessing steps like hierarchical reachability/grounding. citeturn1search12turn1search6turn1search24

**Standardized languages and benchmarks**  
HDDL was proposed to address the lack of a common input language in hierarchical planning, with explicit motivation around domain-independent planners and system comparability/integration. citeturn16search8turn16search0  
HTN competition tracks use HDDL as input. citeturn16search1turn16search20turn16search5

### Major variants and extensions of HTN planning

The following list is restricted to variants explicitly formalized and/or repeatedly referenced in hierarchical planning literature/tooling.

**Totally ordered vs partially ordered HTN**  
Totally ordered fragments are often treated as prominent in benchmarks and tooling; plan verification for totally ordered HTN has connections to parsing/context-free grammar membership, and dedicated verification approaches exploit this structure. citeturn16search6turn16search22turn16search2

**Task Insertion HTN (TIHTN) and extensions**  
TIHTN allows insertion of actions outside the decomposition hierarchy, hybridizing classical planning with HTN planning and addressing incomplete hierarchies. citeturn1search1turn1search4  
Further extensions incorporate constraints (e.g., TIHTNS adds state constraints). citeturn1search7

**Hybrid planning (HTN + causal links / POCL ideas)**  
Work on complexity of “hybrid planning” treats the combination of HTN-style decomposition with partial-order causal link structures as a distinct formalism and analyzes tight bounds across subclasses. citeturn1search25

**Goal-hierarchy formalisms related to HTN**  
Hierarchical Goal Network (HGN) planning decomposes goals rather than tasks and is explicitly positioned as bridging classical planning and hierarchical planning, partly to make classical heuristics easier to incorporate. citeturn1search2turn1search14turn1search11  
Goal-Task Network (GTN) approaches unify goal and task decomposition with task sharing. citeturn1search17turn1search2

**Translations/compilations between HTN and classical planning**  
The HTN-to-PDDL translation line shows restricted HTN classes can be compiled into classical planning encodings and that “small and incomplete” HTN knowledge can improve classical planner performance when translated. citeturn7search0turn7search24  
More recent work continues translating HTN problems to classical planning and evaluating competitiveness. citeturn7search8turn16search35

**Plan repair and verification as first-class problems**  
HTN plan repair via model transformation aims to enable use of off-the-shelf HTN planners for repair by transforming repair into planning. citeturn7search1turn7search9  
HTN plan verification has dedicated lines of work including parsing-based verification and compiling verification into planning. citeturn16search6turn16search2

**Temporal/numeric extensions to hierarchical languages**  
Work proposing “HDDL 2.1” argues HDDL lacks temporal and numeric constraints needed for some “real world applications,” and proposes extending HDDL by drawing from PDDL2.1 and ANML concepts. citeturn0academia29turn0academia31turn2search3

**Hierarchical planning under nondeterminism and uncertainty**  
Recent work claims the first approach to finding strong solutions for fully observable nondeterministic (FOND) HTN problems, using relaxations/compilations to deterministic HTN to reuse deterministic grounders/heuristics. citeturn1search5turn10search12

### Canonical non-game applications (examples)

**Web service composition**  
HTN planning (using SHOP2) was applied to automatic composition of OWL-S web services; the paper argues HTN is suitable for service composition and presents implementation details. citeturn0search19turn0search16

**Interactive assistance with planning, repair, execution, explanation**  
A system for assisting home theater assembly combines planning and interaction components to generate, execute, repair, present, and explain plans, and reports an empirical evaluation. citeturn7search2turn7search30

**Robotics planning/acting integration**  
Hierarchical planning integrated with task and motion planning (“planning in the now”) emphasizes top-down hierarchical commitments to reduce search and lessen long-range projection; other robotics work describes integrating planning and acting with dispatching and execution monitoring. citeturn9search2turn7search15turn7search3

### Worked example A (HTN): document automation workflow

This example is intentionally “workflow-engine shaped”: the primitives correspond to external tools/services; the compound tasks represent structured process knowledge.

**Domain primitives (primitive tasks/actions)**  
Let the state be a set of facts; primitive tasks include:

- `FetchDoc(id)`  
  Preconditions: `DocExists(id)`  
  Effects: `DocFetched(id)`

- `ExtractClaims(id)`  
  Preconditions: `DocFetched(id)`  
  Effects: `ClaimsExtracted(id)`

- `CiteSources(claims)`  
  Preconditions: `ClaimsExtracted(id)`  
  Effects: `CitationsAttached(id)`

- `RenderPDF(id)`  
  Preconditions: `CitationsAttached(id)`  
  Effects: `PdfRendered(id)`

These correspond to a STRIPS-like action semantics. citeturn2search0turn9search5

**Compound task**  
`ProduceCitedReport(id)`

**Methods (decomposition alternatives)**  
- `M1: ProduceCitedReport(id)`  
  Preconditions/guards: `DocType(id, "web")`  
  Decomposes into:  
  `FetchDoc(id) ≺ ExtractClaims(id) ≺ CiteSources(id) ≺ RenderPDF(id)`

- `M2: ProduceCitedReport(id)`  
  Preconditions/guards: `DocType(id, "local")`  
  Decomposes into:  
  `LoadLocalDoc(id) ≺ ExtractClaims(id) ≺ CiteSources(id) ≺ RenderPDF(id)`

This illustrates HTN’s “method library” as the central authoring artifact. The meaning of the plan is not merely “achieves a goal predicate,” but “is reachable via permitted decompositions for ProduceCitedReport.” citeturn9search5turn1search12

**Execution trace and repair points**  
If `CiteSources` fails due to insufficient data (a model mismatch between extraction and citation), HTN repair work frames repair relative to the remaining hierarchy and executed prefix, and discusses transforming repair problems so off-the-shelf planners can be used. citeturn7search1turn7search23

## GOAP deep dive

### Rigorous conceptual model

GOAP in widely cited game AI practice is a planning architecture that:
- maintains a representation of current world state as a set of facts/variables,
- defines goals as desired world-state conditions (and typically priorities),
- defines actions with preconditions/effects and action costs,
- uses search to sequence actions that achieve the selected goal from the current state,
- executes stepwise and replans when the current plan becomes invalid or better goals arise. citeturn11view0turn12view2turn14view0turn14view1

A direct statement in the “Three States and a Plan” document describes world state as a conjunction of literals / assignment to variables. citeturn11view0turn12view2  
That same source explicitly frames A* as the “old friend” used after introducing action costs to guide search toward a lowest-cost sequence of actions to satisfy a goal. citeturn11view0turn12view2turn6search0

A separate practitioner-oriented planning chapter describes GOAP as implementing practical planning with “actions as C++ classes,” “plans as paths in a space of states,” and “search as path planning in a space of states,” with many implementations applying actions backward from goal to initial state, and notes forward search variants. citeturn14view0turn13view0

### Precise definition set (requested deliverable)

**Goal (in GOAP)**  
A desired condition over world state (often a partial assignment). In game GOAP slides and writeups, goals are frequently expressed as desired world-state variables/facts. citeturn16search23turn11view0

**Goal selection**  
A method that chooses a goal based on current world state and priority/utility-like values; the Orkin account explicitly discusses goals competing and falling back to lower-priority goals when a plan cannot be formed. citeturn11view0turn12view2turn12view1

**Action modeling**  
An action *a* is modeled by:
- preconditions `Pre(a)` (facts that must hold to apply the action),
- effects `Eff(a)` defining how the world state changes,
- cost `Cost(a)` used to compute plan cost. citeturn12view2turn14view0turn2search0

**Heuristic choice**  
GOAP commonly uses heuristic graph search (A* or variants). A* theory provides conditions under which the heuristic yields optimal minimum-cost paths given admissibility; GOAP transfers this to the action graph. citeturn6search0turn12view2turn14view0

**World state progression**  
Applying an action updates the state using its effects, often described in STRIPS-like add/delete terms or equivalent set operations. citeturn2search0turn13view0turn14view1

**Plan**  
A totally ordered sequence of actions (common in game GOAP implementations) representing a path in the state space from current state to a state satisfying the goal; a practitioner chapter explicitly calls a GOAP-like plan “a totally ordered set of actions.” citeturn13view0turn14view1

**Execution loop**  
A plan is executed action-by-action. When an action fails or when new information invalidates assumptions, the system replans. The Orkin scenario explicitly describes repeated replanning after failures (door won’t open ⇒ kick ⇒ alternate entry). citeturn12view2turn11view0

### How GOAP differs from “pure STRIPS” in game practice

Orkin explicitly describes GOAP as inspired by STRIPS planning while making changes for real-time practicality, including adding cost per action and adding procedural preconditions/effects; the document also claims elimination of add/delete lists as an implementation difference in that specific system. citeturn16search27turn12view2turn11view0

The practitioner chapter emphasizes implementation and optimization details that are not central in academic planning papers but are architecturally relevant: representing actions in data files for designer iteration, choosing search procedures (e.g., forward BFS vs backward regression), and optimizing memory/time via compact predicate encoding and data structures. citeturn13view0turn14view1

### Major variants and extensions of GOAP

Unlike HTN, GOAP is not a single standardized formalism with a unified competition language. The “variant set” is therefore best described as recurring engineering patterns documented in game AI practice and hybrids.

**Forward vs backward search**  
The GOAP-like planning chapter describes backward application from the goal state as a common pattern and mentions forward-search variants that appear easier to debug. citeturn13view0turn14view0

**Representation optimizations**  
Compact bitset encodings for state predicates, predicate indexing, and parameter unification optimizations are described as main levers to keep planning within real-time budgets. citeturn13view0turn14view1

**GOAP + reactive execution hybrids**  
Academic game AI work explicitly positions planners (including GOAP and HTN) as suited for longer-term planning and Behavior Trees for reactive acting, motivating hybrid approaches. citeturn8search3turn2search4turn8search28

**Utility-driven goal selection**  
Game utility theory materials formalize scoring-based decision selection among options (utility calculations, considerations, etc.). Such systems are frequently used to pick goals or action “intent” that is then realized by planners like GOAP. citeturn8search27turn8search5turn8search2

**Multi-agent GOAP**  
There are specialized studies and systems for multi-agent cooperation using GOAP-like approaches (typically focusing on coordination and performance tradeoffs). citeturn8search14turn16search38

### Worked example B (GOAP): tactical behavior selection with replanning

This example abstracts Orkin’s described “layering behaviors” and “dynamic problem solving” claims into a minimal GOAP model.

**World state facts (subset)**  
`HasCover`, `EnemyVisible`, `InMeleeRange`, `DoorBlocked`, `HasWindowEntry`, `Alive`, `Reloaded`.

**Goals (prioritized)**  
- `G1: Survive` (desired: `Alive=true` and optionally `HasCover=true`)  
- `G2: KillEnemy` (desired: `EnemyDead=true`)  
Orkin describes layering goals such as Cover and KillEnemy and emphasizes that the system discovers dependencies at runtime through preconditions/effects. citeturn12view1turn12view2

**Actions**  
- `GotoCover`  
  Preconditions: `HasCoverLocation=true`  
  Effects: `HasCover=true`  
  Cost: low/moderate

- `AttackFromCover`  
  Preconditions: `HasCover=true ∧ EnemyVisible=true ∧ Reloaded=true`  
  Effects: `EnemyDead=true` (simplified deterministic effect for illustration)  
  Cost: moderate

- `MeleeAttack`  
  Preconditions: `InMeleeRange=true`  
  Effects: `EnemyDead=true`  
  Cost: moderate

- `OpenDoor`  
  Preconditions: `DoorClosed=true`  
  Effects: `DoorOpen=true`  
  Cost: low

- `KickDoor`  
  Preconditions: `DoorClosed=true`  
  Effects: `DoorOpen=true`  
  Cost: higher

- `EnterViaWindow`  
  Preconditions: `HasWindowEntry=true`  
  Effects: `InsideRoom=true`  
  Cost: higher

Orkin’s text includes the “door blocked ⇒ kick ⇒ window” replanning narrative, indicating execution updates working memory/world state and invokes replanning when actions fail. citeturn12view2turn11view0

**Execution monitoring**  
Upon `OpenDoor` failure, set `DoorBlocked=true` in working memory and trigger replanning; the narrative explicitly describes this as the source of dynamic problem solving. citeturn12view2turn11view0

## Comparative analysis

### Representation, planning flow, search behavior, monitoring, authoring

**Representation**
- HTN: primary representation is a task hierarchy plus decomposition methods that generate constrained task networks; primitive actions still require state semantics. citeturn9search5turn1search12  
- GOAP: primary representation is a factored world state plus actions with preconditions/effects/costs and goals as desired world-states; planners search for a minimal-cost action sequence. citeturn11view0turn12view2turn14view0turn14view1

**Planning flow**
- HTN: top-down refinement from abstract tasks to primitive actions via method selection; planning can interleave with acting, but the hierarchy constrains what “counts” as a solution. citeturn9search5turn7search3turn7search23  
- GOAP: pick a goal, then run a state-space search (often framed as pathfinding in state space) to find a plan; execute and replan as needed. citeturn12view2turn14view0turn6search0

**Search behavior**
- HTN: search is over decompositions (method choices, task orderings, variable bindings), and may be structured as progression search, plan-space search, compilations, or SAT/BDD approaches; its branching is often dominated by method choice and applicable decompositions. citeturn1search12turn1search6turn1search18turn1search25  
- GOAP: search is over states/actions like classical planning; branch factor is the number of applicable actions; planning cost is sensitive to state representation and action set size; practice-focused material discusses explicit optimization for time and memory budgets. citeturn14view0turn14view1turn6search0

**Execution monitoring and replanning**
- HTN: repair and replanning are complicated by hierarchical constraints and executed prefixes; there is explicit HTN plan repair literature and explicit integration of acting and planning over hierarchical operational models. citeturn7search1turn7search3turn7search23  
- GOAP: game-practice descriptions explicitly trigger replanning when failures occur or new information arrives; this is typically framed as updating working memory/world state and rerunning the planner. citeturn12view2turn11view0

**Authoring style**
- HTN: authoring centers on method libraries and task decomposition structure (procedural knowledge encoding). HDDL standardization work and hierarchical planning surveys treat model engineering as a core cost/benefit axis. citeturn16search8turn1search12turn15search5  
- GOAP: authoring centers on action definitions and goal predicates; Orkin emphasizes decoupling goals and actions to avoid “embedded FSM per goal” complexity, and practitioner chapters discuss representing actions as data files to enable non-programmer iteration. citeturn12view0turn14view0turn13view0

image_group{"layout":"carousel","aspect_ratio":"16:9","query":["hierarchical task network planning decomposition diagram","goal oriented action planning GOAP action graph diagram","behavior tree vs planner diagram","HTN vs GOAP comparison diagram"],"num_per_query":1}

### Comparison matrix (requested deliverable)

The table entries are phrased as structural properties or as claims explicitly grounded in cited sources.

| Axis | HTN (typical properties) | GOAP (typical properties) |
|---|---|---|
| Expressiveness | Can be more expressive than classical STRIPS planning; complexity depends strongly on restrictions; rich control knowledge encoded via methods. citeturn0search3turn1search12 | Aligns with classical planning (STRIPS-like) with costs; expressiveness depends on action language used and whether procedural effects are allowed in practice. citeturn2search0turn16search27turn14view0 |
| Determinism assumptions | Baseline HTN planning formalism is deterministic in action effects; extensions exist for nondeterminism (e.g., FOND HTN). citeturn1search5turn10search12 | Baseline GOAP implementations in games typically assume deterministic effects or rely on replanning when reality differs; richer nondeterministic semantics generally move toward FOND/MDP-style planning. citeturn12view2turn10search4turn10search12 |
| Authoring burden | Requires explicit task hierarchy and methods; domain modeling tooling supports validation (HDDL and IDE validation tool). citeturn16search8turn15search5turn1search12 | Requires action library with preconditions/effects and goal definitions; practitioner guidance emphasizes data-driven action authoring and optimization for iteration. citeturn12view0turn14view0turn13view0 |
| Runtime cost | Highly dependent on method branching and planner style; domain-independent heuristics and preprocessing exist in modern frameworks. citeturn1search6turn1search24turn1search18 | Sensitive to action set size and state representation; practitioner sources describe time/memory optimization as central to practical GOAP. citeturn14view0turn14view1 |
| Explainability | Plan trace includes decomposition structure, enabling “why” explanations tied to chosen methods; explicit explain/repair work exists in assistance systems. citeturn7search2turn7search1 | Plans are action sequences with costs; explainability typically comes from goal choice plus action chain; Orkin emphasizes “decoupled goals/actions” and working memory as shared context. citeturn12view0turn11view0 |
| Adaptability | Methods constrain allowed behavior; flexibility can be increased via TIHTN/task insertion and via repair/resume planning variants. citeturn1search1turn7search23turn7search25 | Adaptation via replanning is a core practice narrative; action set/goal set modularity supports behavior variation by composition. citeturn12view2turn12view0turn11view0 |
| Partial failure handling | Dedicated HTN plan repair literature and comparisons; repair is shaped by definitions of what it means to “repair” under hierarchical constraints. citeturn7search25turn7search1turn7search17 | Common pattern is detect failure → update world state → replan; this is not unique to GOAP but appears explicitly in GOAP game accounts. citeturn12view2turn11view0 |
| Concurrency support | Partially ordered HTN supports concurrency at the representation level; temporal/numeric extensions are under active discussion (HDDL 2.1 proposals). citeturn0academia31turn1search12turn2search3 | Most GOAP implementations treat plans as totally ordered sequences; concurrency is usually handled by separate systems (schedulers/BTs) unless extending to temporal planning. citeturn14view1turn8search3 |
| Observability | Hierarchical planning research includes plan verification, debugging, and tooling around model validation; execution monitoring is emphasized in acting/planning integration. citeturn16search2turn15search5turn7search3 | Observability is implementation-defined; some game practice includes working memory shared between goals/actions and emphasizes debugging via decoupled structure, but lacks a standardized observability formalism. citeturn12view0turn11view0turn14view0 |
| Fit for long-horizon agents | Designed to encode long-horizon structure via hierarchy; robotics and assistance systems explicitly use hierarchical models for extended tasks and propose interleaving acting/planning. citeturn9search2turn7search3turn7search2 | Designed for “practical planning” under budgets; long-horizon support scales with action modeling and search constraints; typically used for short-to-medium horizon sequences in games, with hybrids for reactivity. citeturn14view0turn8search3turn8search28 |

## Relationship to adjacent planning and execution paradigms

This section frames HTN/GOAP relative to commonly used adjacent paradigms requested.

### Classical planning and STRIPS-style action systems

GOAP’s core modeling choices (state as set of facts, actions with preconditions/effects, planning as finding a sequence of actions to reach a goal) align with the STRIPS tradition and classical planning formulations. citeturn2search0turn12view2turn14view0  
As documented in game practice, GOAP is explicitly described as differing from STRIPS in certain implementation details for real-time use (e.g., action costs, procedural conditions). citeturn16search27turn12view2

HTN planning in the modern planning literature is often presented as “classical planning plus hierarchical decomposition knowledge,” where primitive actions still rely on STRIPS-like semantics but the solution space is constrained by allowable decompositions and task-network constraints. citeturn9search5turn1search12turn0search16

### PDDL ecosystems and planning competitions

PDDL was created to standardize planning problem descriptions for competitions and has evolved via competition-driven requirements (e.g., PDDL2.1 for temporal/numeric domains). citeturn2search6turn2search11turn2search3  
HDDL extends PDDL for hierarchical planning problems and is used as a common language for hierarchical planning tracks. citeturn16search8turn16search20turn16search1  
The existence of dedicated model validation tooling for HDDL indicates the hierarchical planning ecosystem treats authoring correctness as a first-class engineering issue. citeturn15search5turn15search12

### Behavior Trees, utility systems, and state machines

Behavior Trees are a hierarchical control structure for switching between tasks/actions and are characterized (in robotics/game literature) as modular and reactive; formal analysis tools and stochastic extensions exist. citeturn2search4turn2search12  
Statecharts extend finite state machines with hierarchy and concurrency and were introduced as a formalism for complex reactive systems. citeturn3search0turn3search11  
Utility theory approaches score options and select actions/goals via computed utilities; game industry materials document “utility theory” and design patterns for utility-based decision making. citeturn8search5turn8search23turn8search27

Hybrid architectures combining planners (GOAP/STRIPS/HTN) with Behavior Trees are explicitly motivated in game AI research as a way to achieve both long-term deliberation and reactive execution. citeturn8search3turn8search28turn8search13

### Workflow DAGs, BPMN-style workflows, and rule engines

Workflow DAG systems represent workflows as directed acyclic graphs of tasks with dependencies (e.g., Airflow’s description of DAGs as the workflow model). citeturn4search4turn4search0  
BPMN standardizes graphical notation and semantics for business process modeling, including constructs for processes and collaborations. citeturn3search2turn3search31  
Rule engines use pattern matching over working memory to trigger rule firings; the Rete algorithm is a canonical pattern-match algorithm for production systems. citeturn3search1turn3search5

**Structural relationship to HTN/GOAP**
- DAG workflows: primarily encode a fixed partial order (often static at runtime). GOAP/HTN are plan generators that can *synthesize* sequences/structures rather than merely execute a predefined graph, though HTN methods can be seen as generating constrained partial orders. citeturn4search4turn9search5turn1search12  
- BPMN: serves as a modeling/communication specification; execution semantics depend on engines. HTN resembles a *generative* hierarchical process model; GOAP resembles a *goal-seeking* plan generator. citeturn3search2turn9search5turn12view2  
- Rule engines: provide reactive inference/control based on working memory; GOAP and HTN can also rely on working memory/state, but they produce multi-step plans/policies rather than single-step rule firings. citeturn3search5turn11view0turn9search5

## Practical systems design implications, failure modes, evaluation, and bibliography

This section consolidates: practical implications, failure modes and mitigation patterns, evaluation criteria and decision matrix, recommended heuristics, open questions, and annotated bibliography. The framing is “useful for later architecture synthesis.”

### Translation of game-oriented explanations into software architecture language (requested deliverable)

Game GOAP writeups use terms such as “working memory,” “goal/action decoupling,” and “replanning.” These map to standard software architecture constructs as follows:

- **World state / working memory** → a shared mutable state store (in-memory snapshot), or a derived projection from an event log; Orkin explicitly contrasts black-box goal FSMs with shared working memory accessible to goals/actions. citeturn11view0turn12view0turn5search6turn5search2  
- **Actions** → idempotent (or explicitly compensated) side-effecting operations: API calls, tool invocations, file edits, database writes. Practical planning advice that models actions as code units (e.g., C++ classes) corresponds to encapsulating tool calls behind typed interfaces. citeturn14view0turn5search33  
- **Goal selection** → a policy layer that chooses a target condition, which can be driven by priorities, utility scoring, SLA constraints, or deadlines. citeturn12view2turn8search27turn8search5  
- **Plan** → an executable trace (sequence) of operations; in durable execution systems, execution is recorded as event history (commands/events), enabling replay and recovery. citeturn14view1turn3search7  
- **Replanning** → re-running synthesis against updated state/projections; in environments with nondeterministic tools, replanning is analogous to retry+branch logic governed by updated observations. citeturn12view2turn10search12turn5search33

HTN-specific translation:
- **Methods** → reusable process fragments / “procedures” with preconditions; comparable to workflow templates, runbooks, or typed subroutines, but explicitly available to a planning algorithm for refinement. citeturn9search5turn16search8  
- **Compound tasks** → business-level intents or work packages; primitive tasks correspond to tool calls. citeturn9search5turn7search2  
- **Task networks** → partially ordered workflow graphs produced by decomposition; align with DAG execution models when restricted to acyclic partial orders. citeturn9search5turn4search4

### How HTN and GOAP change with nondeterministic, model-driven, expensive, or state-revealing actions (requested deliverable)

**Nondeterministic action outcomes (fully observable)**  
Classical deterministic planning (including typical GOAP and baseline HTN) assumes actions have predictable effects. When actions are nondeterministic, the planning problem becomes policy synthesis rather than fixed-sequence synthesis: in FOND planning, a solution is typically a policy that prescribes actions contingent on reached states, and concepts such as “strong” and “strong cyclic” solutions capture guarantees under nondeterminism/fairness assumptions. citeturn10search12turn10search4turn10search0  
Hierarchical variants exist: recent work introduces/claims approaches to strong solutions for FOND HTN problems. citeturn1search5turn10search12

**Partial observability and state-revealing actions**  
Under partial observability, planning is performed in belief space (sets/distributions of possible states) and conditional plans branch on observations/sensing actions; strong planning under partial observability is defined and solved via AND-OR search in belief space in classical literature. citeturn10search2turn10search34turn10search18  
For GOAP-like systems in software tools, this corresponds to actions that reveal state only after execution (e.g., call API → learn permissions; run tests → learn failing subset). Without explicit belief modeling, the common engineering response is to replan after observation; this is analogous to online contingent planning methods that select useful sensing actions and interleave planning with execution. citeturn10search26turn12view2

**Expensive actions**  
When actions are expensive (latency, cost, risk), both HTN and GOAP require cost models that match operational objectives. GOAP already centers cost as a planner input. citeturn12view2turn14view0  
For HTN planning, “acting and planning” integration work frames decision steps as selecting among refinement/acting choices, optionally using a planner for near-optimal advice under a utility function. citeturn7search3turn7search11  
In workflow systems, expensive actions also implicate retry policy and idempotency; safe retries depend on idempotent APIs or deduplication keys. citeturn5search33turn5search9

**Model-driven actions and model mismatch**  
Both approaches assume a model of action effects. Model mismatch arises when the modeled preconditions/effects diverge from reality; HTN plan repair work treats this as a central cause of execution failure and contrasts repair vs replanning. citeturn7search1turn7search5  
In GOAP game practice, the response is typically to replan using updated working memory. citeturn12view2turn11view0

### Telemetry, durable state, and artifact lineage in hierarchical planning (requested deliverable)

**Telemetry as state estimation**  
Planning depends on a state representation; in dynamic environments, state is inferred from telemetry/events. Durable execution systems record event histories and replay workflow code deterministically to reconstruct state; this is a concrete mechanism for maintaining a consistent execution trace over long horizons. citeturn3search7turn3search32

**Durable state as the substrate for replanning/repair**  
HTN plan repair and acting/planning integration assume access to execution history and a current state estimate; durable execution event histories supply this history. citeturn7search1turn7search3turn3search7

**Artifact lineage**  
For document and code workflows, outputs (artifacts) must be traceable to inputs, transformations, and responsible agents. PROV-DM defines a provenance data model using entities, activities, and agents, intended for interoperable interchange of provenance records. citeturn5search3turn5search11  
Extending PROV with workflow structure has been explicitly studied, indicating direct relevance to recorded multi-step processes. citeturn5search23

**Interaction with hierarchical planning**  
Hierarchical planning produces structured execution traces: decomposition choices, method applications, and primitive execution. Capturing these as provenance “activities” linked to produced “entities” (artifacts) yields an audit-friendly lineage that supports debugging and postmortems and can be used by repair algorithms that require knowledge of what has been executed. citeturn7search1turn5search3turn5search23

### Failure modes and mitigation patterns (requested deliverable)

The failure modes below are framed as categories; mitigation patterns are stated as design constraints or mechanisms and linked to relevant mature sources.

**Stale world state**
- Failure mode: planning is performed on a stale or inconsistent state snapshot; action applicability/effects assumptions fail at runtime.  
- Mitigations:
  - Maintain an explicit execution trace/event history and reconstruct state from it (durable execution/replay). citeturn3search7turn3search32  
  - Use execution monitoring and plan repair rather than assuming single-shot planning; HTN plan repair literature treats model inaccuracies and execution failures as a central case. citeturn7search1turn7search23  
  - For partial observability domains, model sensing actions / belief updates when guarantees are needed. citeturn10search2turn10search26

**Over-decomposition (HTN-specific risk)**
- Failure mode: method libraries encode excessive micro-structure, increasing authoring burden and brittleness to change; also increases the surface area for mismatches.  
- Mitigations:
  - Introduce task insertion or hybridization when hierarchies are incomplete or too rigid, acknowledging that this changes the allowed solution space. citeturn1search1turn1search22  
  - Use tooling and validation for hierarchical models (HDDL + HDDL Parser) to reduce structural errors (e.g., cycles, invalid constructs). citeturn15search5turn15search12turn16search8

**Search blowup**
- Failure mode: combinatorial explosion from many actions (GOAP) or many method choices/parameterizations (HTN).  
- Mitigations:
  - GOAP: use representation/indexing optimizations (bitsets, predicate indexing), and constrain action set sizes; practitioner materials treat time/memory profiling and optimization as required steps. citeturn13view0turn14view1  
  - HTN: use hierarchical reachability/grounding preprocessing (e.g., task decomposition graphs) and heuristic guidance in modern frameworks. citeturn1search24turn1search6turn1search18  
  - Use translations/compilations when advantageous (HTN→PDDL) to leverage classical heuristics/search. citeturn7search0turn7search24

**Hidden side effects**
- Failure mode: actions have side effects not reflected in the planner’s state model; repeated execution (due to retries) causes duplication or corruption.  
- Mitigations:
  - Enforce idempotent operation design for retried actions (idempotency keys / server-side deduplication); AWS guidance explicitly frames idempotent APIs as a mitigation for undesirable retry side effects. citeturn5search33turn5search9  
  - Use saga-style compensation for long-lived multi-step processes where atomic rollback is not feasible; the saga notion decomposes a long-lived transaction into a sequence of transactions with compensating steps. citeturn5search4turn5search0

**Poor authoring ergonomics**
- Failure mode: models become hard to evolve; errors are hard to detect; coupling to a planner/framework is high.  
- Mitigations:
  - Adopt standardized languages and validation toolchains where possible (HDDL, HDDL Parser). citeturn16search8turn15search5  
  - For GOAP-like systems, keep actions data-driven to enable iteration without recompilation, as described in practical planning chapters. citeturn14view0turn13view0

**Nondeterministic tools**
- Failure mode: action outcomes vary; plans fail or partially complete; naive retry loops induce cascades.  
- Mitigations:
  - Formalize outcome uncertainty when guarantees matter (FOND planning / contingent planning); otherwise explicitly adopt replan-on-observation semantics and monitor for loops. citeturn10search12turn10search2turn12view2  
  - Use bounded retries and explicit retry policies with idempotency safeguards. citeturn5search33turn5search9

**Partial completion**
- Failure mode: some actions succeed and commit side effects before later failure; state becomes “in between plans.”  
- Mitigations:
  - Use saga-style compensation and “repair vs replan” strategies; HTN plan repair work explicitly separates repair from restarting planning, and saga work provides the compensating-transaction structure. citeturn7search1turn5search4  
  - Maintain an append-only event history and reconstruct projections; event sourcing empirical studies characterize benefits and challenges (including rebuilding projections and evolution). citeturn5search2turn5search6

**Observability gaps**
- Failure mode: insufficient visibility into why the planner chose a plan, why execution diverged, or what state was assumed.  
- Mitigations:
  - Record both planning-time artifacts (selected goal/task, decomposition choices, heuristic/cost evaluations where feasible) and execution-time events; workflow systems explicitly record command/event histories for replay. citeturn3search7turn5search3turn7search2  
  - Use provenance standards (PROV) to tie artifacts to generation activities/agents. citeturn5search3turn5search11turn5search23

### Evaluation framework and decision matrix (requested deliverable)

This framework is phrased as evaluation criteria against which HTN, GOAP, or hybrids can be assessed for an agentic execution platform.

**Correctness and guarantees**
- Deterministic correctness: does the planner’s model correspond to actual action semantics? (model mismatch rate; failure rate). citeturn7search1turn12view2  
- Nondeterministic guarantees: if needed, is the system solving a policy problem (FOND/contingent), and which guarantee class is targeted (strong vs strong cyclic)? citeturn10search12turn10search0turn10search2

**Modeling and authoring cost**
- Time to add a new action/tool and validate it; availability of static validation (types, cycles, missing symbols). citeturn15search5turn13view0  
- Time to add/modify task decompositions and assess reachability/grounding impact (HTN). citeturn1search24turn16search8

**Runtime performance**
- Median and tail latency for planning; memory footprint; sensitivity to branching factor and action set size; explicit profiling/tracing strategy. citeturn13view0turn1search6

**Operational resilience**
- Idempotency, retries, deduplication, compensation (saga). citeturn5search33turn5search4  
- Ability to resume after crash using durable state/event history. citeturn3search7turn3search32

**Observability and audit**
- Presence of explicit execution trace with replay; provenance metadata for artifacts. citeturn3search7turn5search3turn5search23  
- Ability to explain plan choices; HTN assistance systems include plan explanation as a delivered feature. citeturn7search2turn11view0

**Change management**
- Model evolution impact: how often planning models change; whether old executions must remain reproducible; event-sourced systems report schema evolution as a major challenge and develop tactics (versioning, upcasting, etc.). citeturn5search2turn3search7

**Decision matrix (condensed)**  
- Prefer HTN when: operational procedures are constrained (compliance/runbooks), hierarchical structure is stable and reusable, and explanation/repair require mapping failures back to structured intent. citeturn7search2turn7search1turn1search12  
- Prefer GOAP when: the domain is best modeled as modular actions with costs and the desired sequences should be discovered by search from current state to goal, with frequent replanning under changing contexts. citeturn12view2turn14view0turn6search0  
- Prefer a hybrid when: long-horizon intent requires explicit structure (HTN or goal decomposition) but execution must remain reactive (BT/state-machine layer), matching the planner-vs-BT separation discussed in hybrid game AI work. citeturn8search3turn2search4turn7search3

### Recommended design heuristics (requested deliverable)

These heuristics are stated as “if-conditions” rather than prescriptions.

- If the domain requires that only certain procedural decompositions are acceptable (policy/compliance/safety), use HTN-style explicit method constraints to define the admissible plan space, and treat classical goals as insufficient to capture admissibility. citeturn9search5turn0search3turn1search12  
- If the domain is naturally described as a set of reusable tools/actions with well-defined preconditions/effects and a cost model, and acceptable solutions are any that reach the goal state, use GOAP/classical planning search as the primary synthesis mechanism. citeturn2search0turn12view2turn14view0  
- If the environment is dynamic and long-running, implement explicit plan monitoring with replan/repair triggers, and persist execution traces in a replayable history to make monitoring and recovery operationally reliable. citeturn12view2turn7search1turn3search7  
- If tools are nondeterministic or state-revealing, decide explicitly whether the platform requires policy-level guarantees (FOND/contingent) or accepts replan-on-observation behavior; the former implies policy synthesis and the latter implies robust state refresh and idempotent retries. citeturn10search12turn10search2turn5search33  
- If actions have irreversible side effects, require idempotency or compensations at the action interface; sagas provide a decomposition and compensation pattern for long-lived transactions. citeturn5search4turn5search33  
- If hierarchical models are used, adopt shared languages and validation tooling to reduce model engineering errors and to support multi-planner portability assumptions. citeturn16search8turn15search5

### Worked example C (hybrid): coding agent task with deterministic workflow substrate and model-driven steps

This example demonstrates why a hybrid is frequently superior for modern tool-using agents that must mix deterministic steps (build/test) with model-driven steps (issue interpretation).

**Problem**  
“Fix repository bug described in an issue; produce a verified patch.”

This problem class is used as an evaluation target in SWE-bench, which frames real-world software engineering as a challenging sustained testbed for language-model-based agents. citeturn6search7turn6search3

**Hybrid architecture (conceptual)**  
- High-level intent decomposition (HTN-like): `FixIssue(issue_id)` decomposes into `Reproduce`, `Localize`, `Patch`, `Validate`, `Submit`.  
- Within `Localize` and `Patch`, use GOAP-like planning to select sequences of tool actions (search, open file, edit, run unit tests) that satisfy subgoals such as “failing test identified” or “all tests pass.”  
- Execute via a deterministic workflow runtime with durable event history (for crash recovery and reproducibility). citeturn7search3turn12view2turn3search7turn6search7

**GOAP-like action set (subset)**  
- `SearchRepo(query)` → effects: `SearchResultsAvailable`  
- `OpenFile(path)` → effects: `FileOpen(path)`  
- `EditFile(path, diff)` → effects: `FileModified(path)`  
- `RunTests()` → effects: `TestResult(pass|fail)` (nondeterministic/time-varying in practice due to environment)  
- `ApplyFix()` → effects: `IssueResolved` (validated by tests)

This aligns with GOAP’s “plans as paths in state space” concept; state predicates are derived from tool outputs rather than physical sensors. citeturn14view0turn12view2turn6search7

**Execution trace and lineage**  
Store each tool invocation and output as an event; store produced artifacts (patches, logs) with provenance links (entity/activity/agent) per PROV concepts. citeturn3search7turn5search3turn5search23

**Failure handling**  
- If `RunTests()` fails after `EditFile`, the workflow is partially complete; use compensation (revert) or branch to `Localize` again; retried calls must be idempotent or deduplicated. citeturn5search4turn5search33turn12view2

### Open questions and unresolved tensions (requested deliverable)

**Learning and maintaining hierarchical models**  
Hierarchical planning workshops explicitly list automated learning/synthesis of hierarchical models and use of generative AI/LLMs for hierarchical planning/modeling as current topics, indicating this remains an open research area rather than settled engineering practice. citeturn15search1turn15search2

**LLM integration with hierarchical planning**  
ChatHTN and related work (including extensions/learning work built on ChatHTN) propose interleaving symbolic HTN planning with LLM-generated decompositions when methods are missing, with stated soundness properties for produced plans. This is an emerging line that shifts some authoring burden from manual method design to LLM-assisted decomposition while retaining symbolic validation. citeturn15search21turn15search0turn15search13

**Semantic gap between world models and execution reality**  
Both GOAP and HTN abstractions rely on modeled preconditions/effects; plan repair work, acting/planning integration, and provenance/event-history practices address symptoms (repair, monitoring, traceability) but do not eliminate the mismatch problem. citeturn7search1turn7search3turn5search3

**Guarantees under nondeterminism**  
FOND and partial observability planning provide formal solution concepts (strong/strong cyclic, conditional plans) but integrating these guarantees into practical multi-tool agent stacks introduces cost and modeling complexity; hierarchical extensions are an active research area rather than standardized practice. citeturn10search12turn10search2turn1search5turn1search5

**Concurrency semantics**  
HTN naturally represents partial orders; GOAP implementations are typically sequential. Extending hierarchical planning languages with explicit temporal/numeric constraints is actively discussed, indicating a gap between baseline HDDL and operational concurrency needs in some domains. citeturn0academia29turn2search3turn1search12

### Annotated bibliography (requested deliverable)

Entries are grouped by theme and each annotation states the specific contribution relevant to HTN/GOAP system architecture.

**Foundations: classical planning and search**
- entity["book","Automated Planning","ghallab nau traverso 2004"] (theory/practice text referenced widely in planning curricula and later work): establishes classical planning models, algorithms, and extensions; serves as a general reference context for both GOAP (as classical planning adaptation) and HTN (as a distinct formalism). citeturn9search19turn9search12  
- STRIPS original paper: defines operators, preconditions/effects, and goal-based plan search framework. citeturn2search0turn2search13  
- A* original formulation: defines heuristic minimum-cost path search foundations used by GOAP-style planning. citeturn6search0turn6search12  
- Graphplan: introduces planning graphs and shortest partial-order plan generation. citeturn9search3turn9search22  
- PDDL manual + PDDL2.1: define standardized planning domain/problem languages and temporal/numeric semantics. citeturn2search6turn2search11turn2search3

**HTN core and ecosystem**
- HTN planning complexity/expressivity: formal complexity results for HTN planning under varying restrictions; foundational for understanding theoretical limits and fragment selection. citeturn0search3turn0search24  
- UMCP sound/complete procedure for HTNs: formal syntax/semantics and algorithmic foundations for specific HTN fragments. citeturn9search5turn0search18  
- SHOP2: HTN planner that plans in execution order and supports temporal/metric domains; influential in HTN practice narratives. citeturn0search16turn0search4  
- Hierarchical planning survey: taxonomy of hierarchical planning realizations; useful for system designers to enumerate variant semantics and solver approaches. citeturn1search12  
- PANDA framework: modern domain-independent hierarchical planning framework integrating preprocessing and multiple solving approaches. citeturn1search6turn1search24  
- HDDL language proposal: motivates and defines a common hierarchical planning input language based on PDDL for comparability/integration. citeturn16search8turn16search0  
- HDDL Parser demo/tooling: IDE-integrated validation tooling for HDDL models; indicates maturation of model authoring infrastructure. citeturn15search5turn15search12  
- TIHTN and bounds: formalizes task insertion as hybrid planning and analyzes complexity. citeturn1search1turn1search4  
- HTN plan repair (model transformation) and comparative plan repair analyses: formalize repair problems and compare algorithm definitions/behaviors. citeturn7search1turn7search25turn7search17  
- HTN verification: compilation-based verification and parsing-based approaches for totally ordered fragments. citeturn16search2turn16search6turn16search22  
- HTN with nondeterminism: strong solutions for FOND HTN and related probabilistic hierarchical work. citeturn1search5turn1search5turn10search12

**GOAP and game AI planning practice**
- Orkin, “Three States and a Plan”: describes GOAP for F.E.A.R. and presents the decoupling goals/actions, layering behaviors, and replanning narratives; includes explicit world-state-as-literals framing and A*-style search framing for action planning. citeturn12view0turn12view1turn12view2turn11view0  
- Practical GOAP optimization chapter: documents GOAP-like planning as “actions as C++ classes,” “plans as paths in states,” backward vs forward search choices, and implementation-level optimization concerns. citeturn14view0turn14view1turn13view0  
- Hybrid planner + Behavior Trees paper (game context): motivates mixing planners (including GOAP/HTN) for long-term deliberation with Behavior Trees for reactive execution. citeturn8search3

**Planning, acting, and workflow-oriented reliability**
- Acting/planning integration with hierarchical operational models: defines an integrated acting and planning system using the same hierarchical operational models and a reactive acting engine, relevant to runtime execution monitoring and decision steps. citeturn7search3turn7search11  
- FOND planning and probabilistic planning languages: define nondeterministic/probabilistic planning representations and solution concepts (policies, strong cyclic). citeturn10search12turn10search5turn10search2  
- Saga pattern: long-lived transaction decomposition into subtransactions and compensations; relevant to partial completion and rollback/compensation in multi-step execution. citeturn5search4turn5search0  
- Idempotent APIs for safe retries: architecture guidance on mitigating retry side effects by idempotent operations. citeturn5search33turn5search9  
- PROV-DM and workflow provenance extensions: provenance representation for entities/activities/agents and extensions capturing workflow structure. citeturn5search3turn5search23  
- Durable execution workflows with event histories: durable workflow execution semantics with deterministic replay and event history records. citeturn3search7turn3search32

**Emerging adaptations to LLM agent systems**
- ReAct: interleaves reasoning traces and tool actions for LLM agents; relevant to plan-act interleaving and exception handling. citeturn6search5turn6search1  
- Reflexion: maintains reflective text memory across trials to improve agent behavior; relevant to learning from execution feedback rather than static planning models. citeturn6search6turn6search2  
- ChatHTN: interleaves symbolic HTN planning and LLM-generated decompositions when methods are missing, with stated soundness; represents a concrete emerging bridge between HTN authoring burden and LLM-generated structure. citeturn15search21turn15search0turn15search6  
- SWE-bench: an evaluation framework for repository-level software engineering tasks that highlights long-horizon, tool-driven complexity. citeturn6search7turn6search3

### Implications For Deterministic Workflow Engines

- Systems should borrow from HTN when: acceptable executions must conform to explicit procedural constraints (runbooks, compliance, safety envelopes), and when hierarchical explanation/repair is required; hierarchical planning and plan repair literature specifically address repair under hierarchical constraints and execution-history dependence. citeturn9search5turn7search1turn7search2  
- Systems should borrow from GOAP when: behavior is best specified as a modular action library with preconditions/effects/costs and goal conditions, and the system should dynamically synthesize short-to-medium action sequences through search and replanning; GOAP game practice is explicitly framed as action planning via A*-style search over world states. citeturn12view2turn14view0turn6search0  
- Systems should avoid both (as primary mechanisms) when: the dominant need is executing already-specified dependency graphs (static DAG workflows) or reactive rule firing (rule engines) rather than synthesizing new multi-step plans; in such cases DAG/BPMN/statecharts/rule systems provide direct execution semantics without the modeling overhead of planning. citeturn4search4turn3search2turn3search5turn3search0