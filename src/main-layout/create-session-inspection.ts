/**
 * Which directory inspection a result belongs to.
 *
 * A project inspection is an answer about one specific path, and two of them can be in flight at
 * once — the user types a path, changes their mind, types another. They do not return in the order
 * they were asked: the second is often on a warm cache while the first is still walking a cold
 * network share. Applying whichever lands last attributes one directory's Git capability to
 * another, and the dialog then offers worktree creation for a folder that is not a repository.
 *
 * Logical cancellation, not transport cancellation. The in-flight IPC call still completes; its
 * result is simply not applied. Pretending the call was aborted would be a claim about the native
 * side that nothing here can make.
 */
export interface InspectionTicket {
  readonly id: number;
  readonly path: string;
}

export interface InspectionTracker {
  /** Claims the next request id for `path`, superseding anything already in flight. */
  begin(path: string): InspectionTicket;
  /**
   * Whether this ticket's result may still be applied.
   *
   * Both halves matter: the id rules out a superseded request, and the path rules out a result
   * arriving for a field the user has since edited back and forth to the same request id.
   */
  accepts(ticket: InspectionTicket, currentPath: string): boolean;
  /** Invalidates everything in flight. Used when the surface opens, closes, or changes mode. */
  reset(): void;
}

export function createInspectionTracker(): InspectionTracker {
  let current = 0;
  return {
    begin(path) {
      current += 1;
      return { id: current, path };
    },
    accepts(ticket, currentPath) {
      return ticket.id === current && ticket.path === currentPath;
    },
    reset() {
      // Advancing rather than zeroing: a ticket held by an in-flight request must never match
      // again, and resetting to zero would let the next request collide with an old ticket.
      current += 1;
    },
  };
}
