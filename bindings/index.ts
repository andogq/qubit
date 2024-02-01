import type  { User } from "./../example/bindings/User";
{ version: (_a: null) => string, user: { get: (_id: string) => User, create: (name: string, email: string, age: number) => User } }