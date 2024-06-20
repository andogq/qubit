export type Mutation<Args extends any[], Return> = {
  mutate: (...args: Args) => Promise<Return>;
};
