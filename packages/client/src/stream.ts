export type StreamSubscriber<T> = ({ on_data, on_error, on_end }: {
	on_data?: (data: T) => void,
	on_error?: (error: Error) => void,
	on_end?: () => void,
}) => () => void;

export type Stream<T> = {
	subscribe: StreamSubscriber<T>,
};
