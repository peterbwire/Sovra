# Bounded two-slot HTTP scheduling

The Sovra HTTP example now uses two explicit `Slot<String>` values rather than
waiting for one connection to finish before accepting another. Its external
policy grants two stream slots; narrower operator limits still apply.

Each round admits at most one ready connection if a slot is empty, then advances
each active slot once in cyclic order:

1. Poll head preparation; ready heads advance to body preparation next round.
2. Poll body preparation/progress; complete bodies finish checked receive, run
   the handler and prepare one framed response.
3. Poll final output; completion closes custody and empties the slot.

Both occupied slots receive a turn each round. Full application slots suppress
accept calls, leaving additional peers in the OS backlog. Resource-limit refusal
from try-accept skips admission; other accept failures terminate. Connection
failures attempt close, empty that slot and allow the other slot to continue.
Accepted attempts are counted once, including failures; the finite demonstration
stops after its configured count has completed. No unbounded application queue,
worker pool or background task is created.

## Pacing and controls

Strict `{"version":4,"op":"net.yield"}` performs one nominal millisecond sleep,
with callback checks before/after. It accepts no caller-selected duration and
uses no socket/resource slot or grants. A zero network deadline disables it;
unsupported runtime hosts refuse. OS scheduling can overshoot: this is not a hard
one-millisecond completion guarantee. It never resets HTTP accepted-time deadlines
or I/O counters. Source callers remain subject to VM/control limits.

The example yields between rounds to avoid spinning on idle readiness and burning
HTTP I/O allowance as quickly as the CPU permits. It refuses after 5,000 consecutive
rounds with no active connections; this is a turn bound, not an exact wall-clock
idle timeout. Global VM/control limits can stop it earlier. Active connections
retain their original HTTP deadlines and per-attempt I/O limits.

## Evidence and limits

Independent Python clients first withhold a request head while another GET
completes, then finish the withheld head and receive its successful response.
They repeat this with an incomplete fixed-length body while another HEAD finishes.
Both delayed connections must subsequently succeed; dropping slow connections
cannot satisfy the test. The fifteen-connection fixture retains the original
thirteen HTTP/protocol/access cases plus these two delayed peers. CLI and
project-free/Node-free native paths use the same Sovra scheduler. Authenticated
CLI/native clients retain their credential/refusal/admission checks separately.
Host tests cover fixed-schema, zero-deadline refusal, resource neutrality and
cancellation before/after scheduler pacing.

This establishes cooperative I/O interleaving for this bounded example, not hard
latency fairness or a production scheduler. Command-verifier I/O now yields as owned jobs; trusted-provider compatibility
fallbacks and native process launch can still wait; automatic admission/head-preparation
refusals and Continue now advance in one-write polling turns; blocking receive
still uses synchronous refusal output; application handler execution is not preempted between
connections. VM exhaustion/fatal cancellation closes the entire execution scope.
Two clients occupying both slots can delay new peers until completion/refusal;
per-identity admission, larger configurable pools, readiness-driven waiting,
streaming, TLS and deployment controls remain open. Linux remains the qualified
runtime host; Windows compilation alone does not establish runtime support.
