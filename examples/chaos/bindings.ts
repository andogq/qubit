import type { Stream } from "@qubit-rs/client";
export type Metadata = { param_a: string, param_b: number, param_c: boolean, more_metadata: Metadata | null, };
export type MyEnum = "A" | { "B": number } | { "C": { field: number, } } | { "D": NestedStruct };
export type NestedStruct = { a: number, b: boolean, };
export type Test = { a: number, b: boolean, };
export type User = { name: string, email: string, age: number, metadata: Metadata, };

export type QubitServer = { version: () => Promise<string>, count: () => Promise<number>, countdown: (min: number, max: number) => Stream<number>, array: () => Promise<Array<string>>, enum_test: () => Promise<MyEnum>, user: { someHandler: (_id: string) => Promise<User>, create: (name: string, email: string, age: number) => Promise<User>, list: () => Promise<Array<Test>>, asdf: () => Promise<null> } };