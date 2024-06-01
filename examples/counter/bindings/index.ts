
export type QubitServer = { increment: () => Promise<null>, decrement: () => Promise<null>, add: (n: number) => Promise<null>, get: () => Promise<number>, countdown: () => Stream<number> };