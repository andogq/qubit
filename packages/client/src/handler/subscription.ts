export type StreamHandlers<T> = {
  on_data: (data: T) => void;
  on_error: (error: Error) => void;
  on_end: () => void;
};
export type StreamHandler<T> = ((data: T) => void) | Partial<StreamHandlers<T>>;

export type StreamUnsubscribe = () => void;

/**
 * Helper type to add handler to a list of arguments, in a way that it will be named.
 */
type AddHandler<Arr extends any[], Item> = [...Arr, handler: StreamHandler<Item>];

export type Subscription<Args extends any[], Item> = {
  subscribe: (...args: AddHandler<Args, Item>) => StreamUnsubscribe;
};
