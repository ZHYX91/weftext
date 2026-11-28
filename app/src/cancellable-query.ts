import { message, UiError } from "./ui-locale";

/** Reserve a server ticket before sending work so that cancellation can win
 * even when the two HTTP requests arrive out of order. */
export async function cancellableQuery<T>(signal: AbortSignal, prepare: () => Promise<{ runId: string }>,
    execute: (runId: string) => Promise<T>, cancel: (runId: string) => Promise<unknown>): Promise<T> {
    const cancelled = () => new UiError(message("error.api.data.queryCancelled"));
    if (signal.aborted) throw cancelled();
    const { runId } = await prepare();
    if (signal.aborted) {
        await cancel(runId);
        throw cancelled();
    }
    let acknowledgement: Promise<void> | undefined;
    const stop = () => {
        // The run response resolves the race with a result already being delivered.
        acknowledgement = cancel(runId).then(() => {}, () => {});
    };
    signal.addEventListener("abort", stop, { once: true });
    try { return await execute(runId); }
    finally {
        signal.removeEventListener("abort", stop);
        await acknowledgement;
    }
}

