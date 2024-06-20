export type Query<Args extends any[], Return> = {
  query: (...args: Args) => Promise<Return>;
};
