export type KnowledgeReadOutcome =
    | { status: "installed" }
    | { status: "retired" }
    | { status: "failed"; error: unknown };

interface ReadTicket<Input> {
    input: Input;
    completion: Promise<KnowledgeReadOutcome>;
    settle(outcome: KnowledgeReadOutcome): void;
}

/** One actual read and one latest demand, with an independent outcome per demand.
 * The owner freezes inputs, qualifies delivery and decides when callers may join.
 * Retiring a demand never releases its still-running execution slot. */
export class KnowledgeReadQueue<Input> {
    private active: ReadTicket<Input> | null = null;
    private desired: ReadTicket<Input> | null = null;

    constructor(private readonly execute: (input: Input) => Promise<boolean>) {}

    get current(): Input | null { return this.desired?.input ?? null; }

    join(): Promise<KnowledgeReadOutcome> | null { return this.desired?.completion ?? null; }

    invalidate(): void {
        const retired = this.desired;
        this.desired = null;
        retired?.settle({ status: "retired" });
    }

    /** A new request always has a new identity, even after returning to equal inputs. */
    request(input: Input): Promise<KnowledgeReadOutcome> {
        this.invalidate();
        let settle!: ReadTicket<Input>["settle"];
        const completion = new Promise<KnowledgeReadOutcome>(resolve => { settle = resolve; });
        this.desired = { input, completion, settle };
        this.pump();
        return completion;
    }

    private pump(): void {
        if (this.active || !this.desired) return;
        this.active = this.desired;
        void this.run(this.active);
    }

    private async run(ticket: ReadTicket<Input>): Promise<void> {
        let outcome: KnowledgeReadOutcome;
        try {
            outcome = await this.execute(ticket.input) ? { status: "installed" } : { status: "retired" };
        } catch (error) {
            outcome = { status: "failed", error };
        }
        if (this.desired === ticket) {
            this.desired = null;
            ticket.settle(outcome);
        }
        this.active = null;
        this.pump();
    }
}
