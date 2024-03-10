import type { Stream } from "@rstrpc/client";
type Metadata = { param_a: string, param_b: number, param_c: boolean, more_metadata: Metadata | null, };
type User = { name: string, email: string, age: number, metadata: Metadata, };
export type Server = { version: () => Promise<string>, count: () => Promise<number>, countdown: (min: number, max: number) => Stream<number>, user: { get: (_id: string) => Promise<User>, create: (name: string, email: string, age: number) => Promise<User> } };
