import type { Stream } from "@qubit-rs/client";
export type Metadata = { param_a: string, param_b: number, param_c: boolean, more_metadata: Metadata | null, };
export type Test = { a: number, b: boolean, };
export type User = { name: string, email: string, age: number, metadata: Metadata, };

export type Server = { version: () => Promise<string>, count: () => Promise<number>, countdown: (min: number, max: number) => Stream<number>, array: () => Promise<Array<string>>, user: { someHandler: (_id: string) => Promise<User>, create: (name: string, email: string, age: number) => Promise<User>, list: () => Promise<Array<Test>>, asdf: () => Promise<null> } };