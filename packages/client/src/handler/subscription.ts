export type StreamHandlers<T> = {
  on_data: (data: T) => void;
  on_error: (error: Error) => void;
  on_end: () => void;
};
export type StreamHandler<T> = ((data: T) => void) | Partial<StreamHandlers<T>>;

export type StreamSubscriber<T> = (handler: StreamHandler<T>) => () => void;
export type StreamUnsubscribe = () => void;

export type Subscription<T> = {
  subscribe: T;
};
