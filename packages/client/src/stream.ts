export type StreamSubscriber<T> = (
  handler:
    | ((data: T) => void)
    | {
        on_data?: (data: T) => void;
        on_error?: (error: Error) => void;
        on_end?: () => void;
      },
) => () => void;

export type Stream<T> = {
  subscribe: StreamSubscriber<T>;
};
